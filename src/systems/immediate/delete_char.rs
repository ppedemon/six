use ropey::Rope;
use std::ops::Range;

use crate::{
    active_session_and_buffer,
    components::{Coords, EditorCtx, MutBuffer, Registers},
    systems::{
        commons::{char_idx_to_coords, coords_to_char_idx, curr_line},
        insert::Damage,
        nav::{NormalNav, goto_col, utils::ensure_cursor_inside_line},
    },
};

// Delete chars at the current cursor position (x command)
pub fn delete(ctx: &mut EditorCtx, reg: Option<char>, reps: usize) -> Damage {
    let rng = calc_delete_range(ctx, reps);
    let damage = small_delete(ctx, reg, reps, rng);
    ensure_cursor_inside_line(ctx);
    damage
}

// Delete chars behind the current cursor position (X command)
pub fn backspace(ctx: &mut EditorCtx, reg: Option<char>, reps: usize) -> Damage {
    let rng = calc_backspace_range(ctx, reps);
    small_delete(ctx, reg, reps, rng)
}

fn small_delete(ctx: &mut EditorCtx, reg: Option<char>, reps: usize, rng: Range<usize>) -> Damage {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    record_small_delete(&mut ctx.registers, reg, buffer.rope(), rng.clone());

    buffer.edit().remove(rng.clone());

    let row = buf_view.cursor.row;
    buf_view
        .display_buf
        .patch_range(&ctx.config, buffer.rope(), row..row + 1);

    let cursor = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, rng.start);
    goto_col::<NormalNav>(&ctx.config, buffer.rope(), buf_view, cursor.col);

    Damage::Line(buf_view.cursor.row)
}

fn calc_delete_range(ctx: &mut EditorCtx, reps: usize) -> Range<usize> {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let row = buf_view.cursor.row;
    let start_col = buf_view.cursor.col;
    let mut end_col = start_col;

    let line = curr_line(&ctx.config, buffer.rope(), buf_view);
    let mut it = line
        .graphemes_between(start_col, line.display_width)
        .enumerate();

    while let Some((i, (_, span))) = it.next() {
        if i >= reps {
            break;
        }
        end_col = span.end;
    }

    let start_coords = Coords::new(row, start_col);
    let end_coords = Coords::new(row, end_col);

    let start_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, start_coords);
    let end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, end_coords);

    start_idx..end_idx
}

fn calc_backspace_range(ctx: &mut EditorCtx, reps: usize) -> Range<usize> {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let row = buf_view.cursor.row;
    let end_col = buf_view.cursor.col;
    let mut start_col = end_col;

    let line = curr_line(&ctx.config, buffer.rope(), buf_view);
    let mut it = line.rev_graphemes_between(start_col, 0).enumerate();

    while let Some((i, (g, span))) = it.next() {
        if i >= reps {
            break;
        }
        start_col = span.start;
    }

    let start_coords = Coords::new(row, start_col);
    let end_coords = Coords::new(row, end_col);

    let start_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, start_coords);
    let end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, end_coords);

    start_idx..end_idx
}

// Store small delete in registers
fn record_small_delete(
    registers: &mut Registers,
    reg: Option<char>,
    rope: &Rope,
    range: Range<usize>,
) {
    let deleted = rope.slice(range).to_string();
    registers.record_small_delete(reg, deleted);
}
