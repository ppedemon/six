use ropey::Rope;

use crate::{
    active_session_and_buffer,
    cmd::Cmd,
    components::{EditorCtx, MutBuffer, UndoScope, UndoSummary},
    systems::{commons::char_idx_to_coords, insert::Damage, nav::utils::ensure_cursor_inside_line},
};

pub fn undo(ctx: &mut EditorCtx, cmd: Cmd) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let mut cursor_idx = 0;
    let mut merged_summary = UndoSummary::default();

    for _ in 0..cmd.reps.unwrap_or(1) {
        if let Some(undo_summary) = buffer.edit().undo() {
            cursor_idx = undo_summary.start_idx;
            merged_summary.merge(undo_summary);
        } else {
            break;
        }
    }

    if merged_summary.is_empty() {
        return Damage::Intact;
    }

    let damage = undo_summary_to_damage(merged_summary, buffer.rope());

    match damage {
        Damage::Intact => {}
        Damage::Line(line) => {
            buf_view
                .display_buf
                .patch_range(&ctx.config, buffer.rope(), line..line + 1)
        }
        Damage::Range(start_line, end_line) => {
            buf_view
                .display_buf
                .patch_range(&ctx.config, buffer.rope(), start_line..end_line + 1)
        }
        Damage::From(line) => buf_view.display_buf.destroy_from(line),
    }

    let cursor_idx = cursor_idx.min(buffer.rope().len_chars().saturating_sub(1));
    buf_view.cursor = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, cursor_idx);
    ensure_cursor_inside_line(ctx);

    damage
}

fn undo_summary_to_damage(summary: UndoSummary, rope: &Rope) -> Damage {
    match summary.scope {
        UndoScope::None => Damage::Intact,
        UndoScope::PatchRange {
            start_line,
            end_line,
        } => {
            if start_line == end_line {
                Damage::Line(start_line)
            } else {
                Damage::Range(start_line, end_line)
            }
        }
        UndoScope::DestroyFrom { start_line } => Damage::From(start_line),
    }
}
