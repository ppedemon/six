use crate::{
    active_session_and_buffer,
    cmd::{Arg, Cmd, Motion, MotionMode},
    components::{Coords, EditorCtx, MutBuffer, RegisterData, YankData, YankShape},
    systems::{
        commons::{char_idx_to_coords, coords_to_char_idx, cursor_to_char_idx},
        event,
        immediate::yank::{motion_yank, motion_yank_for_c_cmd},
        insert::Damage,
        nav::{self, NormalNav, utils::ensure_cursor_inside_line},
    },
};

pub fn delete(ctx: &mut EditorCtx, cmd: Cmd) -> Damage {
    gen_delete(ctx, cmd, motion_yank)
}

pub fn delete_for_c_cmd(ctx: &mut EditorCtx, cmd: Cmd) -> Damage {
    gen_delete(ctx, cmd, motion_yank_for_c_cmd)
}

type YankFn = fn(
    &mut EditorCtx,
    Motion,
    usize,
    usize,
    Option<MotionMode>,
) -> Option<(RegisterData, YankData)>;

pub fn gen_delete(ctx: &mut EditorCtx, cmd: Cmd, yank_fn: YankFn) -> Damage {
    match cmd.arg {
        Arg::Motion { reps, mode, motion } => {
            let cmd_reps = cmd.reps.unwrap_or(1);
            let arg_reps = reps.unwrap_or(1);
            match yank_fn(ctx, motion, cmd_reps, arg_reps, mode) {
                None => Damage::Intact,
                Some((reg_data, yank_data)) => {
                    let damage = delete_data(ctx, yank_data.shape);
                    ctx.registers.record_delete(cmd.reg, reg_data);
                    ctx.repbuf.save_last_yank(yank_data);
                    damage
                }
            }
        }
        // TODO Implement text-object movement and deletion
        Arg::TextObject { .. } => Damage::Intact,
        Arg::None => Damage::Intact,
    }
}

fn delete_data(ctx: &mut EditorCtx, shape: YankShape) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    match shape {
        YankShape::Char {
            num_lines, end_col, ..
        } => delete_charwise(ctx, num_lines, end_col),
        YankShape::Line { num_lines } => delete_linewise(ctx, num_lines),
        YankShape::Block { rows, cols } => delete_blockwise(ctx, rows, cols),
    }
}

fn delete_charwise(ctx: &mut EditorCtx, lines: usize, end_col: usize) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let cursor = buf_view.cursor;
    let start_idx = cursor_to_char_idx(&ctx.config, buf_view, buffer.rope());
    let end_coords = Coords::new(cursor.row + lines - 1, end_col);
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

fn delete_linewise(ctx: &mut EditorCtx, num_lines: usize) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let is_empty = buf_view.cursor.row == 0 && num_lines >= buffer.rope().len_lines();
    let start_idx = buffer.rope().line_to_char(buf_view.cursor.row);

    let row = if buf_view.cursor.row + 1 == buffer.rope().len_lines() {
        let end_idx = buffer.rope().len_chars();

        // NOTE: we don't want the text to end with a trailing '\n'. So if we are
        // deleting the last line, remove the '\n' of the line above --which will
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

fn delete_blockwise(ctx: &mut EditorCtx, rows: usize, cols: usize) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let cursor = buf_view.cursor;

    let start_col = cursor.col;
    let end_col = start_col + cols;

    for i in 0..rows {
        let line_idx = buffer.rope().line_to_char(cursor.row + i);
        let line = buf_view
            .display_buf
            .ensure_line(&ctx.config, buffer.rope(), cursor.row + i);

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
                buffer.edit().insert(
                    start_idx,
                    &" ".repeat((lspan.end - lspan.start) - (end_col - start_col)),
                );
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
            buffer.edit().insert(start_idx, &" ".repeat(total));
        }
    }

    buf_view
        .display_buf
        .patch_range(&ctx.config, buffer.rope(), cursor.row..cursor.row + rows);

    ensure_cursor_inside_line(ctx);
    Damage::Range(cursor.row, cursor.row + rows)
}
