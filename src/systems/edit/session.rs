use anyhow::Result;
use hecs::Entity;

use crate::{
    normal::EditOp,
    components::{
        Buffer, BufferView, Config, Coords, EditorCtx, EditorState, ExSession, ExState, Focus,
        Session,
    },
    systems::edit::buffer::{Damage, backspace, delete, enter, insert_char},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageEvent {
    buf_id: Entity,
    damage: Damage,
}

impl DamageEvent {
    pub fn intact() -> Self {
        Self {
            buf_id: Entity::DANGLING,
            damage: Damage::Intact,
        }
    }

    pub fn new(buf_id: Entity, damage: Damage) -> Self {
        Self { buf_id, damage }
    }
}

pub fn handle_edit(ctx: &EditorCtx, op: EditOp) -> Result<()> {
    let (focus, session_id) = {
        let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
        (editor.focus, editor.session_id)
    };

    match focus {
        Focus::Ex => handle_ex_edit(ctx, op),
        Focus::Session => {
            let damage_evt = handle_session_edit(ctx, session_id, op)?;
            broadcast_damage(ctx, session_id, damage_evt)
        }
    }
}

fn handle_session_edit(ctx: &EditorCtx, session_id: Entity, op: EditOp) -> Result<DamageEvent> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_session = ctx
        .world
        .query_one::<(&mut Session, &mut BufferView)>(session_id);
    let (session, buf_view) = q_session.get()?;
    let mut buffer = ctx.world.get::<&mut Buffer>(session.buf_id)?;

    session.insert_log.append(op);

    let damage = match op {
        EditOp::InsertChar(c) => insert_char(&config, buf_view, &mut buffer.rope, c),
        EditOp::Tab => insert_char(&config, buf_view, &mut buffer.rope, '\t'),
        EditOp::Enter => enter(&config, buf_view, &mut buffer.rope),
        EditOp::Backspace => backspace(&config, buf_view, &mut buffer.rope),
        EditOp::Delete => delete(&config, buf_view, &mut buffer.rope),
    };

    buffer.dirty = true;
    Ok(DamageEvent::new(session.buf_id, damage))
}

pub fn clear_ex(ctx: &EditorCtx) -> Result<()> {
    let mut q_ex = ctx
        .world
        .query_one::<(&mut ExSession, &mut BufferView)>(ctx.ex_session_id);
    let (ex_session, buf_view) = q_ex.get()?;

    let len_chars = ex_session.rope.len_chars();
    ex_session.rope.remove(0..len_chars);

    buf_view.display_buf.destroy_from(0);
    buf_view.cursor = Coords::default();
    buf_view.target_col = 0;

    Ok(())
}

fn handle_ex_edit(ctx: &EditorCtx, op: EditOp) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_ex = ctx
        .world
        .query_one::<(&mut ExSession, &mut BufferView)>(ctx.ex_session_id);
    let (ex_session, buf_view) = q_ex.get()?;

    match op {
        EditOp::InsertChar(c) => {
            insert_char(&config, buf_view, &mut ex_session.rope, c);
        }
        EditOp::Tab => {
            insert_char(&config, buf_view, &mut ex_session.rope, '\t');
        }
        EditOp::Backspace => {
            backspace(&config, buf_view, &mut ex_session.rope);
        }
        EditOp::Delete => {
            delete(&config, buf_view, &mut ex_session.rope);
        }
        EditOp::Enter => {}
    };

    ex_session.state = match op {
        EditOp::Enter => ExState::Submit(String::from(&ex_session.rope)),
        _ if ex_session.rope.len_chars() == 0 => ExState::Cancel,
        _ => ExState::Idle,
    };

    Ok(())
}

pub fn broadcast_damage(
    ctx: &EditorCtx,
    active_session_id: Entity,
    damage_evt: DamageEvent,
) -> Result<()> {
    if damage_evt.damage == Damage::Intact {
        return Ok(());
    }

    let config = ctx.world.get::<&Config>(ctx.config_id)?;

    for (session_id, session, buf_view) in ctx
        .world
        .query::<(Entity, &Session, &mut BufferView)>()
        .iter()
    {
        if session_id != active_session_id && session.buf_id == damage_evt.buf_id {
            match damage_evt.damage {
                Damage::Intact => {}
                Damage::Line(row) => {
                    let buffer = ctx.world.get::<&Buffer>(damage_evt.buf_id)?;
                    buf_view
                        .display_buf
                        .patch_range(&config, &buffer.rope, row..row + 1);
                }
                Damage::From(row) => buf_view.display_buf.destroy_from(row),
            }
        }
    }

    Ok(())
}
