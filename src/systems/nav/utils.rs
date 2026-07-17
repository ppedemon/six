use crate::{
    components::{Buffer, Config, EditorCtx},
    systems::{
        commons::{curr_line, mut_active_session_query, snap_coords},
        nav::{move_up, rules::NavRules},
    },
};
use anyhow::Result;

pub fn cursor_up<R: NavRules>(ctx: &EditorCtx) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_ex = mut_active_session_query(ctx)?;
    let (session, buf_view) = q_ex.get()?;
    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;

    move_up::<R>(&config, &buffer.rope, buf_view, 1);
    Ok(())
}

pub fn ensure_cursor_inside_line(ctx: &EditorCtx) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_ex = mut_active_session_query(ctx)?;
    let (session, buf_view) = q_ex.get()?;
    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;

    let col = buf_view.cursor.col;
    let line = curr_line(&config, &buffer.rope, buf_view);
    if col >= line.display_width {
        buf_view.cursor.col = line.display_width.saturating_sub(1);
        snap_coords(&config, &buffer.rope, buf_view, buf_view.cursor);
    }
    Ok(())
}
