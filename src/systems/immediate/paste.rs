use ropey::Rope;
use std::{borrow::Cow, format, panic};

use crate::{
    active_session_and_buffer,
    cmd::Cmd,
    components::{
        Buffer, BufferView, Config, Coords, EditorCtx, Level, MutBuffer, Register, RegisterData,
    },
    systems::{
        commons::{char_idx_to_coords, coords_to_char_idx, curr_line, cursor_to_char_idx},
        event,
        insert::{self, Damage},
        nav::{self, InsertNav, utils::ensure_cursor_inside_line},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMode {
    Before,
    After,
}

pub fn paste(ctx: &mut EditorCtx, cmd: Cmd, mode: PasteMode) -> Damage {
    let reg = cmd.reg;
    let reps = cmd.reps.unwrap_or(1);

    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let r = reg.map_or(Register::Unnamed, Register::from);

    if r == Register::LAST_INSERT {
        return paste_last_insert(ctx, reps, mode);
    }

    let reg_data = ctx.registers.read(r);
    match reg_data {
        None => {
            if let Register::Named(name) = r {
                let msg = &format!("Nothing in register {name}");
                ctx.status.set_msg(Level::Error, msg);
            }
            Damage::Intact
        }
        Some(reg_data) => {
            let damage = match reg_data {
                RegisterData::Char { data } => {
                    paste_charwise(&ctx.config, buf_view, buffer, reps, mode, data.as_ref())
                }
                RegisterData::Line { data } => {
                    paste_linewise(&ctx.config, buf_view, buffer, reps, mode, data.as_ref())
                }
                RegisterData::Block { data } => {
                    paste_blockwise(&ctx.config, buf_view, buffer, reps, mode, data)
                }
            };
            event::on_paste(&mut ctx.status, reg_data, reps);
            damage
        }
    }
}

fn paste_last_insert(ctx: &mut EditorCtx, reps: usize, mode: PasteMode) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    if mode == PasteMode::After {
        nav::move_right::<InsertNav>(&ctx.config, buffer.rope(), buf_view, 1);
    }

    // apply_insert_log takes full care of damage control, nothing to do here
    let ops = &ctx.registers.last_insert().to_vec();
    insert::apply_insert_log(ctx, ops, reps);
    ensure_cursor_inside_line(ctx);

    Damage::Intact
}

fn paste_charwise(
    config: &Config,
    buf_view: &mut BufferView,
    buffer: &mut Buffer,
    reps: usize,
    mode: PasteMode,
    data: &str,
) -> Damage {
    let cursor = buf_view.cursor;
    let cursor_idx = cursor_to_char_idx(config, buf_view, buffer.rope());

    let line = curr_line(config, buffer.rope(), buf_view);
    let anchor_col = match line.grapheme_at(cursor.col) {
        None => line.display_width,
        Some((_, span)) => {
            if mode == PasteMode::After {
                span.end
            } else {
                span.start
            }
        }
    };
    let anchor_coords = Coords::new(cursor.row, anchor_col);
    let anchor_idx = coords_to_char_idx(config, buffer.rope(), buf_view, anchor_coords);

    let mut agg_data = String::with_capacity(reps * data.len());
    agg_data.extend(std::iter::repeat(data).take(reps));
    let rope = Rope::from(agg_data);

    buffer.edit().insert_rope(anchor_idx, &rope);

    let damage = if rope.len_lines() <= 1 {
        buf_view
            .display_buf
            .patch_range(config, buffer.rope(), cursor.row..cursor.row + 1);
        Damage::Line(cursor.row)
    } else {
        buf_view.display_buf.destroy_from(cursor.row);
        Damage::From(cursor.row)
    };

    if rope.len_chars() > 0 {
        let new_coords = char_idx_to_coords(
            config,
            buffer.rope(),
            buf_view,
            anchor_idx + rope.len_chars().saturating_sub(1),
        );
        buf_view.cursor = new_coords;
        buf_view.target_col = new_coords.col;
    }

    damage
}

fn paste_linewise(
    config: &Config,
    buf_view: &mut BufferView,
    buffer: &mut Buffer,
    reps: usize,
    mode: PasteMode,
    data: &str,
) -> Damage {
    let char_idx = cursor_to_char_idx(config, buf_view, buffer.rope());
    let line_idx = buffer.rope().char_to_line(char_idx);

    let norm = norm_data(data);
    let norm_ref = norm.as_ref();
    let mut agg_data = String::with_capacity(reps * norm_ref.len());
    agg_data.extend(std::iter::repeat(norm_ref).take(reps));

    let anchor_idx = if mode == PasteMode::Before {
        buffer.rope().line_to_char(line_idx)
    } else {
        if line_idx + 1 < buffer.rope().len_lines() {
            buffer.rope().line_to_char(line_idx + 1)
        } else {
            agg_data.pop();
            let len = buffer.rope().len_chars();
            buffer.edit().insert_char(len, '\n');
            buffer.rope().len_chars()
        }
    };

    let repair_line_idx = if mode == PasteMode::Before {
        line_idx
    } else {
        line_idx + 1
    };

    let rope = Rope::from(agg_data);
    buffer.edit().insert_rope(anchor_idx, &rope);
    buf_view.display_buf.destroy_from(repair_line_idx);

    let new_coords = char_idx_to_coords(config, buffer.rope(), buf_view, anchor_idx);
    buf_view.cursor = new_coords;
    buf_view.target_col = new_coords.col;

    Damage::From(repair_line_idx)
}

fn norm_data(data: &str) -> Cow<'_, str> {
    if !data.ends_with('\n') {
        let mut s = data.to_owned();
        s.push('\n');
        Cow::Owned(s)
    } else {
        Cow::Borrowed(data)
    }
}

fn paste_blockwise(
    config: &Config,
    buf_view: &mut BufferView,
    buffer: &mut Buffer,
    reps: usize,
    mode: PasteMode,
    data: &[String],
) -> Damage {
    let cursor = buf_view.cursor;
    let line = curr_line(config, buffer.rope(), buf_view);

    let anchor_col = match line.grapheme_at(cursor.col) {
        None => line.display_width,
        Some((_, span)) => {
            if mode == PasteMode::After {
                span.end
            } else {
                span.start
            }
        }
    };

    let data = mk_block_data(data, reps);

    let damage = if cursor.row + data.len() > buffer.rope().len_lines() {
        Damage::From(cursor.row)
    } else {
        Damage::Range(cursor.row, cursor.row + data.len())
    };

    // Add missing lines if necessary
    for _ in buffer.rope().len_lines()..cursor.row + data.len() {
        let idx = buffer.rope().len_chars();
        buffer.edit().insert_char(idx, '\n');
    }

    for (i, line) in data.iter().enumerate() {
        let curr_row = cursor.row + i;
        let buf_line = buf_view
            .display_buf
            .ensure_line(config, buffer.rope(), curr_row);

        let line_idx = buffer.rope().line_to_char(curr_row);
        let last_idx = line_idx + buf_line.display_width;

        // buf_line too short, pad until anchor_col with spaces
        if buf_line.display_width <= anchor_col {
            buffer
                .edit()
                .insert(last_idx, &" ".repeat(anchor_col - buf_line.display_width));
            buffer
                .edit()
                .insert(last_idx + anchor_col - buf_line.display_width, &line);
        } else {
            let (_, span) = buf_line.grapheme_at(anchor_col).unwrap();
            let g_idx = line_idx + buf_line.col_to_char_idx(span.start);

            // anchor_col falls inside a wide grapheme:
            //  If the wide grapheme is a tab, break into before and after whitespace
            //  Otherwise, pad initial fragment with spaces and move wide grapheme after pasted data
            if span.start < anchor_col {
                let len_before = anchor_col - span.start;

                if buffer.rope().char(g_idx) == '\t' {
                    buffer.edit().remove(g_idx..g_idx + 1);
                    buffer.edit().insert(g_idx, &" ".repeat(len_before));
                    buffer.edit().insert(g_idx + len_before, &line);
                    buffer.edit().insert(
                        g_idx + len_before + line.len(),
                        &" ".repeat(span.end - anchor_col),
                    );
                } else {
                    buffer.edit().insert(g_idx, &" ".repeat(len_before));
                    buffer.edit().insert(g_idx + len_before, &line);
                }
            } else {
                buffer.edit().insert(g_idx, &line);
            }
        }
    }

    match damage {
        Damage::Range(lo, hi) => buf_view
            .display_buf
            .patch_range(config, buffer.rope(), lo..hi),
        Damage::From(row) => buf_view.display_buf.destroy_from(cursor.row),
        _ => panic!("Impossible damage: {damage:?}"),
    }

    buf_view.display_buf.destroy_from(cursor.row);
    buf_view.cursor = Coords::new(cursor.row, anchor_col);
    buf_view.target_col = anchor_col;

    damage
}

fn mk_block_data(data: &[String], reps: usize) -> Cow<'_, [String]> {
    if reps == 1 {
        Cow::Borrowed(data)
    } else {
        let block_data = data
            .iter()
            .map(|line| line.repeat(reps))
            .collect::<Vec<_>>();
        Cow::Owned(block_data)
    }
}
