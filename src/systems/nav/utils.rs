use crate::{
    components::{Buffer, BufferView, Config, EditorCtx, EditorState, Session}, systems::nav::{move_up, rules::NavRules},
};
use anyhow::Result;

pub fn cursor_up<R: NavRules>(ctx: &EditorCtx) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
    let mut q_ex = ctx
        .world
        .query_one::<(&Session, &mut BufferView)>(editor.session_id);
    let (session, buf_view) = q_ex.get()?;
    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;

    move_up::<R>(&config, &buffer.rope, buf_view, 1);
    Ok(())
}
