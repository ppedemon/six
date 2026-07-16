use crate::{
    components::{Buffer, Config, EditorCtx},
    systems::{
        commons::mut_active_session_query,
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
