use anyhow::Result;
use hecs::Entity;
use ropey::Rope;

use super::{
    buffer,
    rules::{InsertNav, NavRules, NormalNav},
};
use crate::cmd::{Motion, Secondary, Arg};
use crate::components::{
    Buffer, BufferView, Config, EditorCtx, EditorState, ExSession, Focus, Mode, Session, Viewport,
};

pub struct NavArgs {
    pub motion: Motion,
    pub reps: Option<usize>,
    pub arg: Arg,
}

impl NavArgs {
    pub fn new(motion: Motion, reps: Option<usize>, arg: Arg) -> Self {
        Self {
            motion,
            reps,
            arg,
        }
    }
}

pub fn handle_nav(ctx: &EditorCtx, nav_args: NavArgs) -> Result<()> {
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
    match editor.focus {
        Focus::Ex => handle_ex_nav(ctx, nav_args),
        Focus::Session => handle_session_nav(ctx, editor.session_id, nav_args),
    }
}

fn handle_ex_nav(ctx: &EditorCtx, nav_args: NavArgs) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_ex = ctx
        .world
        .query_one::<(&ExSession, &mut BufferView)>(ctx.ex_session_id);
    let (ex_session, buf_view) = q_ex.get()?;

    let NavArgs { motion, reps, .. } = nav_args;
    let reps = reps.unwrap_or(1);

    match motion {
        Motion::Left => {
            if buf_view.cursor.col > 1 {
                buffer::move_left::<InsertNav>(&config, &ex_session.rope, buf_view, reps);
            }
        }
        Motion::Right => {
            buffer::move_right::<InsertNav>(&config, &ex_session.rope, buf_view, reps);
        }
        _ => {}
    }

    Ok(())
}

fn handle_session_nav(ctx: &EditorCtx, session_id: Entity, nav_args: NavArgs) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_session = ctx
        .world
        .query_one::<(&mut Session, &mut BufferView)>(session_id);
    let (session, buf_view) = q_session.get()?;
    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;

    match session.mode {
        Mode::Insert => {
            session.insert_log.reset();
            session_nav::<InsertNav>(
                &config,
                &buffer.rope,
                &mut session.viewport,
                buf_view,
                nav_args,
            );
        }
        Mode::Normal => {
            session_nav::<NormalNav>(
                &config,
                &buffer.rope,
                &mut session.viewport,
                buf_view,
                nav_args,
            );
        }
    }

    Ok(())
}

const PAGE_SCROLL_MARGIN: u16 = 3;

fn session_nav<R: NavRules>(
    config: &Config,
    rope: &Rope,
    viewport: &mut Viewport,
    buf_view: &mut BufferView,
    nav_args: NavArgs,
) {
    let NavArgs {
        motion,
        reps,
        arg,
    } = nav_args;
    let reps = default_reps(&nav_args);

    match motion {
        Motion::Up => buffer::move_up::<R>(config, rope, buf_view, reps),
        Motion::Down => buffer::move_down::<R>(config, rope, buf_view, reps),
        Motion::Left => buffer::move_left::<R>(config, rope, buf_view, reps),
        Motion::Right => buffer::move_right::<R>(config, rope, buf_view, reps),
        Motion::PageUp => {
            let pg_size = viewport.pg_size(PAGE_SCROLL_MARGIN);
            buffer::page_up::<R>(config, rope, buf_view, reps, pg_size);
            viewport.scroll_to_row(buf_view.cursor.row);
        }
        Motion::PageDown => {
            let pg_size = viewport.pg_size(PAGE_SCROLL_MARGIN);
            buffer::page_down::<R>(config, rope, buf_view, reps, pg_size);
            viewport.scroll_to_row(buf_view.cursor.row);
        }
        Motion::NextBigWord => buffer::next_big_word(config, rope, buf_view, reps),
        Motion::NextSubWord => buffer::next_sub_word(config, rope, buf_view, reps),
        Motion::PrevBigWord => buffer::prev_big_word(config, rope, buf_view, reps),
        Motion::PrevSubWord => buffer::prev_sub_word(config, rope, buf_view, reps),
        Motion::EndBigWord => buffer::end_big_word(config, rope, buf_view, reps),
        Motion::EndSubWord => buffer::end_sub_word(config, rope, buf_view, reps),

        Motion::FirstNonBlankInLine => buffer::line_first_non_blank::<R>(config, rope, buf_view),
        Motion::StartOfLine => buffer::start_of_line::<R>(config, rope, buf_view),
        Motion::EndOfLine => buffer::end_of_line::<R>(config, rope, buf_view),
        Motion::FirstNonBlankInFile => buffer::file_first_non_blank::<R>(config, rope, buf_view),
        Motion::StartOfFile => buffer::start_of_file::<R>(config, rope, buf_view),
        Motion::EndOfFile => buffer::end_of_file::<R>(config, rope, buf_view),

        Motion::BigGotoLine => goto_line::<R>(config, rope, viewport, buf_view, reps),
        Motion::SmallGotoLine => {
            if let Arg::Secondary(Secondary::GotoLine) = arg {
                goto_line::<R>(config, rope, viewport, buf_view, reps);
            }
        }
    }
}

fn default_reps(args: &NavArgs) -> usize {
    args.reps.unwrap_or_else(|| match args.motion {
        Motion::BigGotoLine => usize::MAX,
        _ => 1,
    })
}

// On startup, move cursor to the first non-blank character of the active session
pub fn init_cursor_pos(ctx: &EditorCtx) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;

    let mut q_session = ctx
        .world
        .query_one::<(&Session, &mut BufferView)>(editor.session_id);
    let (session, buf_view) = q_session.get()?;

    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;
    buffer::file_first_non_blank::<NormalNav>(&config, &buffer.rope, buf_view);

    Ok(())
}

pub fn goto_line<R: NavRules>(
    config: &Config,
    rope: &Rope,
    viewport: &mut Viewport,
    buf_view: &mut BufferView,
    line: usize,
) {
    let old_line = buf_view.cursor.row;
    buffer::goto_line::<R>(config, rope, buf_view, line);

    let h = viewport.area.height.saturating_div(2) as usize;
    let scroll_start = viewport.scroll.row;
    let scroll_end = viewport.scroll.row + viewport.area.height as usize;

    if buf_view.cursor.row + h <= scroll_start
        || buf_view.cursor.row.saturating_sub(h) >= scroll_end
    {
        viewport.scroll.row = buf_view.cursor.row.saturating_sub(h).min(
            rope.len_lines()
                .saturating_sub(viewport.area.height as usize),
        );
    }
}
