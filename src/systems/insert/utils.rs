use crate::{
    cmd::EditOp,
    components::{Buffer, BufferView, Config, EditorCtx, EditorState, Session},
    systems::insert::{
        buffer::enter,
        session::{DamageEvent, broadcast_damage},
    },
};
use anyhow::Result;

pub fn open_line(ctx: &EditorCtx) -> Result<()> {
    let (session_id, damage_evt) = {
        let config = ctx.world.get::<&Config>(ctx.config_id)?;
        let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
        let mut q_session = ctx
            .world
            .query_one::<(&mut Session, &mut BufferView)>(editor.session_id);
        let (session, buf_view) = q_session.get()?;
        let mut buffer = ctx.world.get::<&mut Buffer>(session.buf_id)?;

        session.insert_log.append(EditOp::Enter);
        buffer.dirty = true;
        
        let damage = enter(&config, buf_view, &mut buffer.rope);
        let damage_evt = DamageEvent::new(session.buf_id, damage);

        (editor.session_id, damage_evt)
    };

    broadcast_damage(ctx, session_id, damage_evt)
}
