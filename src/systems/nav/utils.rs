use crate::{
    components::EditorCtx,
    systems::{
        commons::{curr_line, snap_coords},
        nav::{move_up, rules::NavRules},
    },
};

pub fn cursor_up<R: NavRules>(ctx: &mut EditorCtx) {
    let (session, buf_view) = ctx.sessions.get_mut(&ctx.editor.session_id).unwrap();
    let buffer = ctx.buffers.get(&session.buf_id).unwrap();
    move_up::<R>(&ctx.config, &buffer.rope, buf_view, 1);
}

pub fn ensure_cursor_inside_line(ctx: &mut EditorCtx) {
    let (session, buf_view) = ctx.sessions.get_mut(&ctx.editor.session_id).unwrap();
    let buffer = ctx.buffers.get(&session.buf_id).unwrap();

    let col = buf_view.cursor.col;
    let line = curr_line(&ctx.config, &buffer.rope, buf_view);
    if col >= line.display_width {
        buf_view.cursor.col = line.display_width.saturating_sub(1);
        snap_coords(&ctx.config, &buffer.rope, buf_view, buf_view.cursor);
    }
}
