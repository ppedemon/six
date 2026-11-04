use ropey::Rope;

use crate::{
    active_session_and_buffer, cmd::Cmd, components::{EditorCtx, MutBuffer, UndoTxnMeta}, systems::{commons::char_idx_to_coords, insert::Damage, nav::utils::ensure_cursor_inside_line},
};

pub fn undo(ctx: &mut EditorCtx, cmd: Cmd) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let mut cursor_idx = 0;
    let mut merged_txn_meta = UndoTxnMeta::default();

    for _ in 0..cmd.reps.unwrap_or(1) {
        if let Some(txn_meta) = buffer.edit().undo() {
            cursor_idx = txn_meta.start_idx;
            merged_txn_meta.merge(txn_meta);
        } else {
            break;
        }
    }

    let damage = txn_meta_to_damage(merged_txn_meta, buffer.rope());
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

fn txn_meta_to_damage(txn_meta: UndoTxnMeta, rope: &Rope) -> Damage {
    if txn_meta.is_empty() {
        return Damage::Intact;
    }

    let max_idx = rope.len_chars().saturating_sub(1);
    let start_idx = txn_meta.start_idx.min(max_idx);
    let end_idx = txn_meta.end_idx.min(max_idx);

    let start_line = rope.char_to_line(start_idx);
    let end_line = rope.char_to_line(end_idx);

    if txn_meta.multi_line {
        Damage::From(start_line)
    } else if start_line == end_line {
        Damage::Line(start_line)
    } else {
        Damage::Range(start_line, end_line)
    }
}
