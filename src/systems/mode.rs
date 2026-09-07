use ropey::Rope;

use crate::{
    active_session, active_session_and_buffer,
    cmd::{Cmd, ExMode, InsertPoint},
    components::{BufferView, Config, EditorCtx, Focus, Level, Mode, TextStyle},
    systems::{
        commons,
        insert::{clear_ex, insert_char, post_insert},
    },
};

pub fn enter_insert(ctx: &mut EditorCtx, insert_point: InsertPoint, cmd: Cmd) {
    clear_ex(ctx);

    ctx.status.clear_cmd();
    ctx.status
        .set_styled_msg(Level::Info, TextStyle::Bold, "-- INSERT --");

    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    ctx.editor.focus = Focus::Session;
    ctx.editor.char_at_cursor = None;

    session.mode = Mode::Insert;
    session.insert_log.init();

    apply_insert_point(&ctx.config, buffer.rope(), buf_view, insert_point);
}

pub fn enter_normal(ctx: &mut EditorCtx) {
    clear_ex(ctx);

    ctx.editor.focus = Focus::Session;
    ctx.editor.char_at_cursor = None;

    ctx.status.clear_msg();
    ctx.status.clear_cmd();

    let (session, _) = active_session!(mut ctx);
    let old_mode = session.mode;
    session.mode = Mode::Normal;

    if old_mode == Mode::Insert {
        post_insert(ctx);
    }
}

pub fn enter_ex(ctx: &mut EditorCtx, ex_mode: ExMode) {
    clear_ex(ctx);

    ctx.editor.focus = Focus::Ex;
    ctx.editor.char_at_cursor = None;

    let ex_session = &mut ctx.ex_session;
    let buf_view = &mut ctx.ex_buffer_view;

    let _ = match ex_mode {
        ExMode::Colon => insert_char(&ctx.config, buf_view, &mut ex_session.edit(), ':'),
        ExMode::SearchForward => insert_char(&ctx.config, buf_view, &mut ex_session.edit(), '/'),
        ExMode::SearchBackward => insert_char(&ctx.config, buf_view, &mut ex_session.edit(), '?'),
    };

    apply_insert_point(&ctx.config, &ex_session.rope(), buf_view, InsertPoint::Last);
}

pub fn goto_insert_point(ctx: &mut EditorCtx, insert_point: InsertPoint) {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    apply_insert_point(&ctx.config, buffer.rope(), buf_view, insert_point);
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
