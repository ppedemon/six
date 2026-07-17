use crate::{
    cmd::EditOp,
    components::EditorCtx,
    systems::insert::{
        buffer::enter,
        session::{DamageEvent, broadcast_damage},
    },
};

pub fn open_line(ctx: &mut EditorCtx) {
    let (session, buf_view) = ctx.sessions.get_mut(&ctx.editor.session_id).unwrap();
    let buffer = ctx.buffers.get_mut(&session.buf_id).unwrap();

    session.insert_log.append(EditOp::Enter);
    buffer.dirty = true;

    let damage = enter(&ctx.config, buf_view, &mut buffer.rope);
    let damage_evt = DamageEvent::new(session.buf_id, damage);

    broadcast_damage(ctx, damage_evt);
}
