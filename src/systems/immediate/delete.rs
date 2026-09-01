use crate::{
    active_session_and_buffer,
    cmd::{Arg, Cmd},
    components::{EditorCtx, MutBuffer, RegisterData},
    systems::{
        commons::cursor_to_char_idx,
        event,
        immediate::yank::motion_yank,
        insert::Damage,
        nav::{self, NormalNav, utils::ensure_cursor_inside_line},
    },
};

pub fn delete(ctx: &mut EditorCtx, cmd: Cmd) -> Damage {
    match cmd.arg {
        Arg::Motion { reps, mode, motion } => {
            let cmd_reps = cmd.reps.unwrap_or(1);
            let arg_reps = reps.unwrap_or(1);
            match motion_yank(ctx, motion, cmd_reps, arg_reps, mode) {
                None => Damage::Intact,
                Some(reg_data) => {
                    let damage = delete_data(ctx, &reg_data);
                    notify_delete(ctx, &reg_data);
                    ctx.registers.record_delete(cmd.reg, reg_data);
                    damage
                }
            }
        }
        // TODO Implement text-object movement and deletion
        Arg::TextObject { .. } => Damage::Intact,
        Arg::None => Damage::Intact,
    }
}

fn delete_data(ctx: &mut EditorCtx, reg_data: &RegisterData) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    match reg_data {
        RegisterData::Char { data } => delete_charwise(ctx, data),
        RegisterData::Line { data } => delete_linewise(ctx, data),
        RegisterData::Block { data } => delete_blockwise(ctx, data),
    }
}

fn delete_charwise(ctx: &mut EditorCtx, data: &str) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let cursor = buf_view.cursor;
    let start_idx = cursor_to_char_idx(&ctx.config, buf_view, buffer.rope());
    let len_chars = data.chars().count();

    buffer.edit().remove(start_idx..start_idx + len_chars);

    let damage = if data.lines().count() <= 1 {
        buf_view
            .display_buf
            .patch_range(&ctx.config, buffer.rope(), cursor.row..cursor.row + 1);
        Damage::Line(cursor.row)
    } else {
        buf_view.display_buf.destroy_from(cursor.row);
        Damage::From(cursor.row)
    };

    ensure_cursor_inside_line(ctx);
    damage
}

fn delete_linewise(ctx: &mut EditorCtx, data: &str) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let data_len = data.chars().count();

    let cursor = buf_view.cursor;
    let len = buffer.rope().len_chars();
    let start_idx = buffer.rope().line_to_char(cursor.row);
    let mut row = cursor.row;

    if start_idx + data_len >= len {
        buffer.edit().remove(start_idx.saturating_sub(1)..len);
        nav::move_up::<NormalNav>(&ctx.config, buffer.rope(), buf_view, 1);
        row = row.saturating_sub(1);
    } else {
        buffer.edit().remove(start_idx..start_idx + data_len);
    }

    buf_view.display_buf.destroy_from(row);

    nav::line_first_non_blank::<NormalNav>(&ctx.config, buffer.rope(), buf_view);
    Damage::From(row)
}

// This ended up being SUPER complicated... is it possible to simplify via an alternative approach?
// All options I tried ended up being a bug farm because of off-by-ones and incorrect assumptions.
// As complex as it is, this implementation seems to work.
fn delete_blockwise(ctx: &mut EditorCtx, data: &[String]) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let cursor = buf_view.cursor;

    for (i, line) in data.iter().enumerate() {
        if line.len() == 0 {
            continue;
        }

        let curr_row = cursor.row + i;
        let buf_line = buf_view
            .display_buf
            .ensure_line(&ctx.config, buffer.rope(), curr_row);

        let line_idx = buffer.rope().line_to_char(curr_row);
        let char_idx = buf_line.col_to_char_idx(cursor.col);
        let start_idx = line_idx + char_idx;
        let end_idx = start_idx + line.chars().count().saturating_sub(1);

        let first = line.chars().next().unwrap();
        let last = line.chars().last().unwrap();

        // Trivial case: line matches exactly the slice to be deleted
        if buffer.rope().char(start_idx) == first && buffer.rope().char(end_idx) == last {
            buffer.edit().remove(start_idx..end_idx + 1);
            continue;
        }

        // Oh, my... initial and/or final chars don't match because of wide chars

        // We use spaces to deal with wide chars, so line has to start or end with spaces
        let llen = line.chars().take_while(|c| *c == ' ').count();
        let rlen = line.chars().rev().take_while(|c| *c == ' ').count();
        assert!(llen > 0 || rlen > 0);

        // And the rope has to differ from the line at some end
        let rope_first = buffer.rope().char(start_idx);
        let rope_last = buffer.rope().char(end_idx);
        assert!(first != rope_first || last != rope_last);

        // Left span
        let rope_idx = start_idx - line_idx;
        let col = buf_line.char_idx_to_col(rope_idx);
        let (_, lspan) = buf_line.grapheme_at(col).unwrap();

        // Right span
        let rope_idx = end_idx - line_idx;
        let col = buf_line.char_idx_to_col(rope_idx);
        let (_, rspan) = buf_line.grapheme_at(col).unwrap();

        // Special case: contiguous spaces for a sequence of wide chars
        if llen == line.len() && rlen == line.len() {
            let start_idx = line_idx + buf_line.col_to_char_idx(lspan.start);
            let end_idx = line_idx + buf_line.col_to_char_idx(rspan.end);
            buffer.edit().remove(start_idx..end_idx);
            buffer
                .edit()
                .insert(start_idx, &" ".repeat(rspan.end - lspan.start - llen));
            continue;
        }

        // Deal with left end
        let (pad_left, start_idx) = if first != rope_first {
            if rope_first == '\t' {
                (lspan.end - lspan.start - llen, start_idx)
            } else {
                (0, line_idx + buf_line.col_to_char_idx(lspan.start))
            }
        } else {
            (0, start_idx)
        };

        // Deal with right end
        let (pad_right, end_idx) = if last != rope_last {
            if rope_last == '\t' {
                (rspan.end - rspan.start - rlen, end_idx + 1)
            } else {
                (0, line_idx + buf_line.col_to_char_idx(rspan.end))
            }
        } else {
            (0, end_idx + 1)
        };

        buffer.edit().remove(start_idx..end_idx);

        if pad_left + pad_right > 0 {
            buffer
                .edit()
                .insert(start_idx, &" ".repeat(pad_left + pad_right));
        }
    }

    buf_view.display_buf.patch_range(
        &ctx.config,
        buffer.rope(),
        cursor.row..cursor.row + data.len(),
    );

    ensure_cursor_inside_line(ctx);
    Damage::Range(cursor.row, cursor.row + data.len())
}

fn notify_delete(ctx: &mut EditorCtx, reg_data: &RegisterData) {
    let (_, _, buffer) = active_session_and_buffer!(ctx);
    event::on_delete(&mut ctx.status, reg_data, buffer.rope().len_chars() == 0);
}
