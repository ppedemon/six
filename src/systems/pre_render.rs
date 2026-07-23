use ratatui::layout::Rect;

use crate::components::{Coords, EditorCtx, Viewport};

pub fn pre_render(ctx: &mut EditorCtx, area: Rect) {
    let (session_area, ex_area) = get_areas(ctx, area);

    scroll_sessions(ctx);
    scroll_ex(ctx);

    resize_sessions(ctx, session_area);
    resize_ex(ctx, ex_area);

    sync_sessions(ctx);
    sync_ex(ctx);
}

fn scroll_sessions(ctx: &mut EditorCtx) {
    for (session, buf_view) in ctx.sessions.values_mut() {
        scroll(buf_view.cursor, &mut session.viewport)
    }
}

fn scroll_ex(ctx: &mut EditorCtx) {
    let cursor = ctx.ex_buffer_view.cursor;
    scroll(cursor, &mut ctx.ex_session.viewport);
}

fn resize_sessions(ctx: &mut EditorCtx, area: Rect) {
    for (session, buf_view) in ctx.sessions.values_mut() {
        resize_session(area, buf_view.cursor, &mut session.viewport);
    }
}

fn resize_ex(ctx: &mut EditorCtx, area: Rect) {
    let cursor = ctx.ex_buffer_view.cursor;
    resize_ex_session(area, cursor, &mut ctx.ex_session.viewport);
}

fn get_areas(ctx: &EditorCtx, area: Rect) -> (Rect, Rect) {
    let total_height = area.height as usize;

    let ex_height = ctx
        .ex_session
        .rope()
        .len_lines()
        .clamp(1, total_height.saturating_sub(1));

    let session_height = total_height.saturating_sub(ex_height);

    let session_area = Rect::new(area.x, area.y, area.width, session_height as u16);
    let ex_area = Rect::new(
        area.x,
        area.y + session_height as u16,
        area.width,
        ex_height as u16,
    );

    (session_area, ex_area)
}

fn sync_sessions(ctx: &mut EditorCtx) {
    for (session, buf_view) in ctx.sessions.values_mut() {
        let buffer = ctx.buffers.get(&session.buf_id).unwrap();
        let len_lines = buffer.rope().len_lines();

        let height = session.viewport.area.height as usize;
        let scroll_row = session.viewport.scroll.row;
        let start = scroll_row.saturating_sub(height);
        let end = (scroll_row + height * 2).min(len_lines);

        buf_view
            .display_buf
            .ensure_range(&ctx.config, buffer.rope(), start..end);
    }
}

fn sync_ex(ctx: &mut EditorCtx) {
    let len_lines = ctx.ex_session.rope().len_lines();
    let height = ctx.ex_session.viewport.area.height as usize;
    let start = ctx.ex_session.viewport.scroll.row.saturating_sub(height);
    let end = (ctx.ex_session.viewport.scroll.row + height * 2).min(len_lines);

    ctx.ex_buffer_view
        .display_buf
        .ensure_range(&ctx.config, &ctx.ex_session.rope(), start..end);
}

fn scroll(cursor: Coords, viewport: &mut Viewport) {
    let width = viewport.area.width as usize;
    let height = viewport.area.height as usize;

    if cursor.row < viewport.scroll.row {
        viewport.scroll.row = cursor.row;
    } else if cursor.row >= viewport.scroll.row + height {
        viewport.scroll.row = cursor.row.saturating_sub(height.saturating_sub(1));
    }

    if cursor.col < viewport.scroll.col {
        viewport.scroll.col = cursor.col.saturating_sub(width / 2);
    } else if cursor.col >= viewport.scroll.col + width {
        viewport.scroll.col = (cursor.col + width / 2).saturating_sub(width);
    }
}

fn resize_session(area: Rect, cursor: Coords, viewport: &mut Viewport) {
    if area.width != viewport.area.width {
        let width = area.width as usize;
        if width <= cursor.col.saturating_sub(viewport.scroll.col) {
            viewport.scroll.col = cursor.col.saturating_sub(width / 2);
        }
    }

    if area.height != viewport.area.height {
        let height = area.height;
        let ratio = cursor.row.saturating_sub(viewport.scroll.row) as f64 / height as f64;
        let x_adj = (height as f64 * ratio).floor() as usize;
        viewport.scroll.row = cursor.row.saturating_sub(x_adj);
    }

    viewport.area = area;
}

fn resize_ex_session(area: Rect, cursor: Coords, viewport: &mut Viewport) {
    if area.width != viewport.area.width {
        let width = area.width as usize;
        if width <= cursor.col.saturating_sub(viewport.scroll.col) {
            viewport.scroll.col = cursor.col.saturating_sub(width / 2);
        }
    }

    let height = area.height as usize;
    viewport.scroll.row = cursor.row.saturating_sub(height.saturating_sub(1));

    viewport.area = area;
}
