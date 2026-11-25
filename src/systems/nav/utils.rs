use crate::{
    active_session_and_buffer,
    cmd::InsertPoint,
    components::EditorCtx,
    systems::{
        commons,
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
    let line = commons::curr_line(&ctx.config, buffer.rope(), buf_view);
    if col >= line.display_width {
        buf_view.cursor.col = line.display_width.saturating_sub(1);
        commons::snap_coords(&ctx.config, buffer.rope(), buf_view, buf_view.cursor);
    }
}

pub fn apply_insert_point(ctx: &mut EditorCtx, insert_point: InsertPoint) {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let cursor = buf_view.cursor;
    let line = commons::curr_line(&ctx.config, buffer.rope(), buf_view);
    buf_view.cursor.col = match insert_point {
        InsertPoint::Curr => line.snap_insert_col(cursor.col),
        InsertPoint::Next => line.next_insert_col(cursor.col),
        InsertPoint::First => line.first_insert_non_blank(),
        InsertPoint::Last => line.display_width,
    };
}
