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
        RegisterData::Block { data, idxs } => delete_blockwise(ctx, data, idxs),
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
fn delete_blockwise(ctx: &mut EditorCtx, data: &[String], idxs: &[(usize, usize)]) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let cursor = buf_view.cursor;

    let mut deleted_chars = 0;
    let mut padding = 0;

    for (i, line) in data.iter().enumerate() {
        if line.is_empty() {
            continue;
        }

        let first_c = line.chars().next().unwrap();
        let last_c = line.chars().last().unwrap();

        let (start_idx, end_idx) = {
            let (start_idx, end_idx) = idxs[i];
            (
                start_idx - deleted_chars + padding,
                end_idx - deleted_chars + padding,
            )
        };

        // Trivial case: line matches exactly the slice to be deleted
        if buffer.rope().char(start_idx) == first_c && buffer.rope().char(end_idx - 1) == last_c {
            buffer.edit().remove(start_idx..end_idx);
            deleted_chars += end_idx - start_idx;
            continue;
        }

        // Oh my, initial and/or final chars don't match because of wide chars: must compute padding
        let line_len = line.chars().count();

        let curr_row = cursor.row + i;
        let line_idx = buffer.rope().line_to_char(curr_row);
        let buf_line = buf_view
            .display_buf
            .ensure_line(&ctx.config, buffer.rope(), curr_row);

        let (_, lspan) = buf_line.grapheme_at(cursor.col).unwrap();
        let (_, rspan) = buf_line
            .grapheme_at(buf_line.char_idx_to_col(end_idx - line_idx - 1))
            .unwrap();

        let mut pad = 0;

        if lspan == rspan {
            // Line to delete is included in a single wide grapheme. If it's a tag, padding is whatever overflows the line
            let char_idx = line_idx + buf_line.col_to_char_idx(lspan.start);
            if buffer.rope().char(char_idx) == '\t' {
                pad = lspan.end - lspan.start - line.chars().count();
            }
        } else {
            let mut lspaces = line.chars().take_while(|c| *c == ' ').count();
            let mut rspaces = line.chars().rev().take_while(|c| *c == ' ').count();
            if lspaces == line_len && rspaces == line_len {
                lspaces = lspan.end - cursor.col;
                rspaces = line_len - lspaces;
            }

            let char_idx = line_idx + buf_line.col_to_char_idx(lspan.start);
            if buffer.rope().char(char_idx) == '\t' {
                pad += lspan.end - lspan.start - lspaces;
            }

            let char_idx = line_idx + buf_line.col_to_char_idx(rspan.start);
            if buffer.rope().char(char_idx) == '\t' {
                pad += rspan.end - rspan.start - rspaces;
            }
        }

        buffer.edit().remove(start_idx..end_idx);
        buffer.edit().insert(start_idx, &" ".repeat(pad));

        deleted_chars += end_idx - start_idx;
        padding += pad;
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
