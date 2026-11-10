use ropey::Rope;

use crate::{
    active_session_and_buffer,
    cmd::Cmd,
    components::{EditorCtx, MutBuffer, Status, UndoScope, UndoSummary},
    systems::{
        commons::char_idx_to_coords,
        event,
        insert::Damage,
        nav::{self, NormalNav, utils::ensure_cursor_inside_line},
    },
};

pub enum BufReport {
    Changed(usize),
    Added(usize),
    Removed(usize),
}

pub fn undo(ctx: &mut EditorCtx, cmd: Cmd) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let orig_len = buffer.rope().len_lines();

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
        event::on_empty_undo_stack(&mut ctx.status);
        return Damage::Intact;
    }

    let new_len = buffer.rope().len_lines();
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

    let cursor_idx = cursor_idx.min(buffer.rope().len_chars());
    buf_view.cursor = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, cursor_idx);
    ensure_cursor_inside_line(ctx);

    report_undo(&mut ctx.status, merged_summary, orig_len, new_len);
    damage
}

pub fn undo_line(ctx: &mut EditorCtx) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let line_idx = buffer.edit().undo_line();

    let damage = match line_idx {
        Some(line_idx) => {
            buf_view
                .display_buf
                .patch_range(&ctx.config, buffer.rope(), line_idx..line_idx + 1);
            nav::goto_line::<NormalNav>(&ctx.config, buffer.rope(), buf_view, line_idx + 1);
            Damage::Line(line_idx)
        }
        None => Damage::Intact,
    };

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

fn report_undo(status: &mut Status, summary: UndoSummary, orig_len: usize, new_len: usize) {
    let buf_report = if orig_len == new_len {
        BufReport::Changed(summary.changes as usize)
    } else if orig_len < new_len {
        BufReport::Added(new_len - orig_len)
    } else {
        BufReport::Removed(orig_len - new_len)
    };
    event::on_undo(status, summary.timestamp_ms, summary.change_id, buf_report);
}
