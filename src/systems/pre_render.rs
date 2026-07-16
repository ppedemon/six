use anyhow::Result;
use hecs::Entity;
use ratatui::layout::Rect;

use crate::{
    components::{Buffer, BufferView, Config, Coords, EditorCtx, ExSession, Session, Viewport},
    systems::commons::mut_ex_session_query,
};

pub fn pre_render(ctx: &mut EditorCtx, area: Rect) -> Result<()> {
    let (session_area, ex_area) = get_areas(ctx, area)?;

    scroll_sessions(ctx);
    scroll_ex(ctx)?;

    resize_sessions(ctx, session_area);
    resize_ex(ctx, ex_area)?;

    sync_sessions(ctx)?;
    sync_ex(ctx)?;

    Ok(())
}

fn scroll_sessions(ctx: &mut EditorCtx) {
    let q_sessions = ctx.world.query_mut::<(&mut Session, &mut BufferView)>();
    for (session, buf_view) in q_sessions {
        scroll(buf_view.cursor, &mut session.viewport)
    }
}

fn scroll_ex(ctx: &EditorCtx) -> Result<()> {
    let buf_view = ctx.world.get::<&BufferView>(ctx.ex_session_id)?;
    let cursor = buf_view.cursor;
    let mut ex_session = ctx.world.get::<&mut ExSession>(ctx.ex_session_id)?;
    scroll(cursor, &mut ex_session.viewport);
    Ok(())
}

fn resize_sessions(ctx: &EditorCtx, area: Rect) {
    let mut q_sessions = ctx.world.query::<(&mut Session, &mut BufferView)>();
    for (session, buf_view) in q_sessions.into_iter() {
        resize_session(area, buf_view.cursor, &mut session.viewport);
    }
}

fn resize_ex(ctx: &EditorCtx, area: Rect) -> Result<()> {
    let buf_view = ctx.world.get::<&BufferView>(ctx.ex_session_id)?;
    let cursor = buf_view.cursor;
    let mut ex_session = ctx.world.get::<&mut ExSession>(ctx.ex_session_id)?;
    resize_ex_session(area, cursor, &mut ex_session.viewport);
    Ok(())
}

fn get_areas(ctx: &EditorCtx, area: Rect) -> Result<(Rect, Rect)> {
    let total_height = area.height as usize;

    let ex_session = ctx.world.get::<&ExSession>(ctx.ex_session_id)?;
    let ex_height = ex_session
        .rope
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

    Ok((session_area, ex_area))
}

fn sync_sessions(ctx: &mut EditorCtx) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_sessions = ctx.world.query::<(Entity, &Session)>();

    for (entity, session) in &mut q_sessions {
        let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;
        let len_lines = buffer.rope.len_lines();

        let height = session.viewport.area.height as usize;
        let scroll_row = session.viewport.scroll.row;
        let start = scroll_row.saturating_sub(height);
        let end = (scroll_row + height * 2).min(len_lines);

        let mut buf_view = ctx.world.get::<&mut BufferView>(entity)?;
        buf_view
            .display_buf
            .ensure_range(&config, &buffer.rope, start..end);
    }

    Ok(())
}

fn sync_ex(ctx: &EditorCtx) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_ex = mut_ex_session_query(ctx)?;
    let (ex_session, buf_view) = q_ex.get()?;

    let len_lines = ex_session.rope.len_lines();
    let height = ex_session.viewport.area.height as usize;
    let start = ex_session.viewport.scroll.row.saturating_sub(height);
    let end = (ex_session.viewport.scroll.row + height * 2).min(len_lines);

    buf_view
        .display_buf
        .ensure_range(&config, &ex_session.rope, start..end);

    Ok(())
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
