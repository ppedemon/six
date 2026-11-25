use ropey::Rope;

use crate::{
    active_session_and_buffer,
    cmd::InsertPoint,
    components::{BufferView, Config, EditorCtx},
    systems::{
        commons::{self, curr_line, snap_coords},
        nav::{move_up, rules::NavRules},
    },
};

pub fn cursor_up<R: NavRules>(ctx: &mut EditorCtx) {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    move_up::<R>(&ctx.config, buffer.rope(), buf_view, 1);
}

pub fn ensure_cursor_inside_line(ctx: &mut EditorCtx) {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let buffer = ctx.buffers.get(&session.buf_id).unwrap();

    let col = buf_view.cursor.col;
    let line = curr_line(&ctx.config, buffer.rope(), buf_view);
    if col >= line.display_width {
        buf_view.cursor.col = line.display_width.saturating_sub(1);
        snap_coords(&ctx.config, buffer.rope(), buf_view, buf_view.cursor);
    }
}

pub fn apply_insert_point(
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
