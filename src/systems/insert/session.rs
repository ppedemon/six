use crate::{
    active_session_and_buffer,
    cmd::EditOp,
    components::{BufferId, Coords, EditorCtx, ExState, Focus},
    systems::{
        commons::{char_idx_to_coords, cursor_to_char_idx},
        insert::buffer::{Damage, backspace, delete, enter, insert_char},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageEvent {
    buf_id: BufferId,
    damage: Damage,
}

impl DamageEvent {
    pub fn new(buf_id: BufferId, damage: Damage) -> Self {
        Self { buf_id, damage }
    }
}

pub fn handle_edit(ctx: &mut EditorCtx, op: EditOp) {
    match ctx.editor.focus {
        Focus::Ex => handle_ex_edit(ctx, op),
        Focus::Session => {
            let damage_evt = handle_session_edit(ctx, op);
            broadcast_damage(ctx, damage_evt)
        }
    }
}

fn handle_session_edit(ctx: &mut EditorCtx, op: EditOp) -> DamageEvent {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    session.insert_log.append(op);

    let damage = match op {
        EditOp::InsertChar(c) => insert_char(&ctx.config, buf_view, &mut buffer.rope, c),
        EditOp::Tab => insert_char(&ctx.config, buf_view, &mut buffer.rope, '\t'),
        EditOp::Enter => enter(&ctx.config, buf_view, &mut buffer.rope),
        EditOp::Backspace => backspace(&ctx.config, buf_view, &mut buffer.rope),
        EditOp::Delete => delete(&ctx.config, buf_view, &mut buffer.rope),
    };

    buffer.dirty = true;
    DamageEvent::new(session.buf_id, damage)
}

pub fn clear_ex(ctx: &mut EditorCtx) {
    let ex_session = &mut ctx.ex_session;
    let buf_view = &mut ctx.ex_buffer_view;

    let len_chars = ex_session.rope.len_chars();
    ex_session.rope.remove(0..len_chars);

    buf_view.display_buf.destroy_from(0);
    buf_view.cursor = Coords::default();
    buf_view.target_col = 0;
}

fn handle_ex_edit(ctx: &mut EditorCtx, op: EditOp) {
    let ex_session = &mut ctx.ex_session;
    let buf_view = &mut ctx.ex_buffer_view;

    match op {
        EditOp::InsertChar(c) => insert_char(&ctx.config, buf_view, &mut ex_session.rope, c),
        EditOp::Tab => insert_char(&ctx.config, buf_view, &mut ex_session.rope, '\t'),
        EditOp::Backspace => backspace(&ctx.config, buf_view, &mut ex_session.rope),
        EditOp::Delete => delete(&ctx.config, buf_view, &mut ex_session.rope),
        EditOp::Enter => Damage::Intact,
    };

    ex_session.state = match op {
        EditOp::Enter => ExState::Submit(String::from(&ex_session.rope)),
        _ if ex_session.rope.len_chars() == 0 => ExState::Cancel,
        _ => ExState::Idle,
    };
}

pub fn broadcast_damage(ctx: &mut EditorCtx, damage_evt: DamageEvent) {
    if damage_evt.damage == Damage::Intact {
        return;
    }

    for (session_id, (session, buf_view)) in ctx.sessions.iter_mut() {
        if *session_id != ctx.editor.session_id && session.buf_id == damage_evt.buf_id {
            let buffer = ctx.buffers.get_mut(&session.buf_id).unwrap();

            match damage_evt.damage {
                Damage::Intact => {}
                Damage::Line(row) => {
                    buf_view
                        .display_buf
                        .patch_range(&ctx.config, &buffer.rope, row..row + 1);
                }
                Damage::From(row) => buf_view.display_buf.destroy_from(row),
            }

            // Cursor might end up outside document boundaries in case of deletions, update if necessary
            let mut cursor_idx = cursor_to_char_idx(&ctx.config, buf_view, &buffer.rope);
            if cursor_idx >= buffer.rope.len_chars() {
                cursor_idx = buffer.rope.len_chars().saturating_sub(1);
                buf_view.cursor =
                    char_idx_to_coords(&ctx.config, &buffer.rope, buf_view, cursor_idx);
            }
        }
    }
}
