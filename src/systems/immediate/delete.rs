use crate::{
    active_session_and_buffer,
    cmd::{Cmd, MotionMode},
    components::{Coords, EditorCtx, MutBuffer, YankData},
    systems::{
        commons::{
            char_idx_to_coords, coords_to_char_idx, cursor_to_char_idx, display_line,
            next_col_or_display_width,
        },
        event,
        insert::Damage,
        nav::{self, NormalNav, interpret, utils::ensure_cursor_inside_line},
    },
};

pub fn delete(ctx: &mut EditorCtx, cmd: Cmd) -> Damage {
    let Ok((yank_data, reg_data)) = interpret(ctx, cmd) else {
        // TODO Maybe show an error here?
        return Damage::Intact;
    };

    let damage = match yank_data.mode {
        MotionMode::Charwise => delete_charwise(ctx, yank_data),
        MotionMode::Linewise => delete_linewise(ctx, yank_data),
        MotionMode::Blockwise => delete_blockwise(ctx, yank_data),
    };

    ctx.registers.record_delete(cmd.reg, reg_data);
    ctx.repbuf.save_last_yank(yank_data);

    damage
}

fn delete_charwise(ctx: &mut EditorCtx, yank_data: YankData) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let lines = yank_data.num_lines();
    let end_col = yank_data.end_col();

    let cursor = buf_view.cursor;
    let start_idx = cursor_to_char_idx(&ctx.config, buf_view, buffer.rope());

    let mut end_coords = Coords::new(cursor.row + lines - 1, end_col);
    if yank_data.inclusive {
        let new_end_col =
            next_col_or_display_width(&ctx.config, buffer.rope(), buf_view, end_coords);
        end_coords.col = new_end_col;
    }
    let end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, end_coords);

    let is_empty = start_idx == 0 && end_idx == buffer.rope().len_chars();
    buffer.edit().remove(start_idx..end_idx);

    let damage = if lines <= 1 {
        buf_view
            .display_buf
            .patch_range(&ctx.config, buffer.rope(), cursor.row..cursor.row + 1);
        Damage::Line(cursor.row)
    } else {
        buf_view.display_buf.destroy_from(cursor.row);
        Damage::From(cursor.row)
    };

    if start_idx < buffer.rope().len_chars() {
        buf_view.cursor = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, start_idx);
    }
    ensure_cursor_inside_line(ctx);

    event::on_delete(&mut ctx.status, lines, is_empty);
    damage
}

fn delete_linewise(ctx: &mut EditorCtx, yank_data: YankData) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let num_lines = yank_data.num_lines();
    let is_empty = buf_view.cursor.row == 0 && num_lines >= buffer.rope().len_lines();
    let start_idx = buffer.rope().line_to_char(buf_view.cursor.row);

    let row = if buf_view.cursor.row + num_lines >= buffer.rope().len_lines() {
        let end_idx = buffer.rope().len_chars();

        // NOTE: we don't want the text to end with a trailing '\n'. So if we are
        // deleting the last line, remove the '\n' of the line above --which would
        // become a trailing '\n' after deleting the last line.
        buffer.edit().remove(start_idx.saturating_sub(1)..end_idx);

        nav::move_up::<NormalNav>(&ctx.config, buffer.rope(), buf_view, 1);
        buf_view.cursor.row.saturating_sub(1)
    } else {
        let end_idx = buffer.rope().line_to_char(buf_view.cursor.row + num_lines);
        buffer.edit().remove(start_idx..end_idx);
        buf_view.cursor.row
    };

    buf_view.display_buf.destroy_from(row);

    nav::line_first_non_blank::<NormalNav>(&ctx.config, buffer.rope(), buf_view);
    event::on_delete(&mut ctx.status, num_lines, is_empty);

    Damage::From(row)
}

fn delete_blockwise(ctx: &mut EditorCtx, yank_data: YankData) -> Damage {
    let (rows, cols) = yank_data.block();
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let cursor = buf_view.cursor;
    let start_col = cursor.col;

    for i in 0..rows {
        let line_idx = buffer.rope().line_to_char(cursor.row + i);
        let line = display_line(&ctx.config, buffer.rope(), buf_view, cursor.row + i);
        let end_col = (start_col + cols).min(line.display_width);

        let Some((lg, lspan)) = line.grapheme_at(start_col) else {
            continue;
        };
        let Some((rg, rspan)) = line.grapheme_at(end_col.saturating_sub(1)) else {
            continue;
        };

        // Special case: the columns to delete fit into a single grapheme
        if lspan.start < start_col && lspan.end > end_col {
            let start_idx = line_idx + line.col_to_char_idx(lspan.start);
            let end_idx = line_idx + line.col_to_char_idx(lspan.end);
            buffer.edit().remove(start_idx..end_idx);
            if lg.chars().all(|c| c == ' ') {
                let n = (lspan.end - lspan.start) - (end_col - start_col);
                buffer
                    .edit()
                    .insert_iter(start_idx, std::iter::repeat_n(' ', n));
            }
            continue;
        }

        let lspaces = if lspan.start < start_col && lg.chars().all(|c| c == ' ') {
            start_col - lspan.start
        } else {
            0
        };

        let rspaces = if rspan.end > end_col && rg.chars().all(|c| c == ' ') {
            rspan.end - end_col
        } else {
            0
        };

        let start_idx = line_idx + line.col_to_char_idx(lspan.start);
        let end_idx = line_idx + line.col_to_char_idx(rspan.end);
        buffer.edit().remove(start_idx..end_idx);

        let total = lspaces + rspaces;
        if total > 0 {
            buffer
                .edit()
                .insert_iter(start_idx, std::iter::repeat_n(' ', total));
        }
    }

    buf_view
        .display_buf
        .patch_range(&ctx.config, buffer.rope(), cursor.row..cursor.row + rows);

    ensure_cursor_inside_line(ctx);
    Damage::Range(cursor.row, cursor.row + rows)
}
