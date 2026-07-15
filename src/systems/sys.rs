use anyhow::Result;
use ropey::Rope;

use crate::{
    cmd::{Cmd, ExMode, InsertPoint, SysOp},
    components::{
        Buffer, BufferView, Config, EditorCtx, EditorState, ExSession, Focus, Level, Mode,
        RepeatBuffer, Session, Status,
    },
    ex::ExRange,
    systems::{
        commons, ex,
        insert::{clear_ex, insert_char, post_insert},
        nav::{NormalNav, move_left},
        quit_editor,
    },
};

pub struct SysArgs {
    pub op: SysOp,
    cmd: Cmd,
}

impl SysArgs {
    pub fn new(op: SysOp, cmd: Cmd) -> Self {
        Self { op, cmd }
    }
}

pub fn handle_sys(ctx: &EditorCtx, args: SysArgs) -> Result<()> {
    match args.op {
        SysOp::EnterNormal => enter_normal(ctx),
        SysOp::EnterInsert(insert_point) => enter_insert(ctx, insert_point, args.cmd),
        SysOp::EnterEx(ex_mode) => enter_ex(ctx, ex_mode),
        SysOp::HardQuit => quit_editor(ctx),
        SysOp::CondWriteAndQuit => {
            ex::save_active(ctx, None, false, true, ExRange::All)?;
            quit_editor(ctx)
        }
    }
}

pub fn enter_insert(ctx: &EditorCtx, insert_point: InsertPoint, cmd: Cmd) -> Result<()> {
    clear_ex(ctx)?;

    let mut status = ctx.world.get::<&mut Status>(ctx.status_id)?;
    status.clear_cmd();
    status.set_msg(Level::Info, "--INSERT--");

    let mut rep_buf = ctx.world.get::<&mut RepeatBuffer>(ctx.repbuf_id)?;
    rep_buf.start_interaction(cmd);

    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut editor = ctx.world.get::<&mut EditorState>(ctx.editor_id)?;
    let mut q_session = ctx
        .world
        .query_one::<(&mut Session, &mut BufferView)>(editor.session_id);
    let (session, buf_view) = q_session.get()?;
    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;

    editor.focus = Focus::Session;
    editor.char_at_cursor = None;

    session.mode = Mode::Insert;
    session
        .insert_log
        .init(cmd.reps.unwrap_or(1).saturating_sub(1));

    apply_insert_point(&config, &buffer.rope, buf_view, insert_point);
    Ok(())
}

pub fn enter_normal(ctx: &EditorCtx) -> Result<()> {
    clear_ex(ctx)?;

    let old_mode = {
        let mut status = ctx.world.get::<&mut Status>(ctx.status_id)?;
        status.clear_msg();
        status.clear_cmd();

        let mut editor = ctx.world.get::<&mut EditorState>(ctx.editor_id)?;
        editor.focus = Focus::Session;
        editor.char_at_cursor = None;

        let mut session = ctx.world.get::<&mut Session>(editor.session_id)?;
        let old_mode = session.mode;
        session.mode = Mode::Normal;

        old_mode
    };

    if old_mode == Mode::Insert {
        post_insert(ctx)?;
    }

    restore_cursor(ctx)
}

fn enter_ex(ctx: &EditorCtx, ex_mode: ExMode) -> Result<()> {
    clear_ex(ctx)?;

    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut editor = ctx.world.get::<&mut EditorState>(ctx.editor_id)?;
    let mut q_ex = ctx
        .world
        .query_one::<(&mut ExSession, &mut BufferView)>(ctx.ex_session_id);
    let (ex_session, buf_view) = q_ex.get()?;

    editor.focus = Focus::Ex;
    editor.char_at_cursor = None;

    let _ = match ex_mode {
        ExMode::Colon => insert_char(&config, buf_view, &mut ex_session.rope, ':'),
        ExMode::SearchForward => insert_char(&config, buf_view, &mut ex_session.rope, '/'),
        ExMode::SearchBackward => insert_char(&config, buf_view, &mut ex_session.rope, '?'),
    };

    apply_insert_point(&config, &ex_session.rope, buf_view, InsertPoint::Last);
    Ok(())
}

fn restore_cursor(ctx: &EditorCtx) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
    let mut q_session = ctx
        .world
        .query_one::<(&mut Session, &mut BufferView)>(editor.session_id);
    let (session, buf_view) = q_session.get()?;
    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;

    let cursor = buf_view.cursor;
    let line = commons::curr_line(&config, &buffer.rope, buf_view);
    buf_view.cursor.col = line.snap_col(cursor.col);
    move_left::<NormalNav>(&config, &buffer.rope, buf_view, 1);

    Ok(())
}

fn apply_insert_point(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    insert_point: InsertPoint,
) {
    let cursor = buf_view.cursor;
    let line = commons::curr_line(&config, rope, buf_view);
    buf_view.cursor.col = match insert_point {
        InsertPoint::Curr => line.snap_insert_col(cursor.col),
        InsertPoint::Next => line.next_insert_col(cursor.col),
        InsertPoint::First => line.first_insert_non_blank(),
        InsertPoint::Last => line.display_width,
    };
}
