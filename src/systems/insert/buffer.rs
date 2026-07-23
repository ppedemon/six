use ropey::Rope;

use crate::components::{BufferView, Config, Coords, MutBuffer};
use crate::systems::commons::{coords_to_char_idx, curr_line, cursor_to_char_idx};
use crate::systems::nav::{InsertNav, move_down, move_right, move_up};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Damage {
    Intact,
    Line(usize),
    From(usize),
}

pub fn insert_char(
    config: &Config,
    buf_view: &mut BufferView,
    text: &mut impl MutBuffer,
    c: char,
) -> Damage {
    let char_idx = cursor_to_char_idx(config, buf_view, text.rope());
    text.insert_char(char_idx, c);
    patch_curr_line(config, text.rope(), buf_view);
    move_right::<InsertNav>(config, text.rope(), buf_view, 1);
    Damage::Line(buf_view.cursor.row)
}

pub fn enter(config: &Config, buf_view: &mut BufferView, text: &mut impl MutBuffer) -> Damage {
    let char_idx = cursor_to_char_idx(config, buf_view, text.rope());
    text.insert_char(char_idx, '\n');
    buf_view.display_buf.destroy_from(buf_view.cursor.row);

    buf_view.cursor.col = 0;
    buf_view.target_col = 0;
    move_down::<InsertNav>(config, text.rope(), buf_view, 1);

    Damage::From(buf_view.cursor.row)
}

pub fn backspace(config: &Config, buf_view: &mut BufferView, text: &mut impl MutBuffer) -> Damage {
    if buf_view.cursor.col > 0 {
        let char_idx = cursor_to_char_idx(config, buf_view, text.rope());

        let cursor = buf_view.cursor;
        let line = curr_line(config, text.rope(), buf_view);
        let prev_col = line.prev_insert_col(cursor.col);
        let prev_idx = coords_to_char_idx(
            config,
            text.rope(),
            buf_view,
            Coords::new(cursor.row, prev_col),
        );

        text.remove(prev_idx..char_idx);
        patch_curr_line(config, text.rope(), buf_view);

        buf_view.cursor.col = prev_col;
        buf_view.target_col = prev_col;

        Damage::Line(buf_view.cursor.row)
    } else if buf_view.cursor.row > 0 {
        join_above(config, buf_view, text)
    } else {
        Damage::Intact
    }
}

pub fn delete(config: &Config, buf_view: &mut BufferView, text: &mut impl MutBuffer) -> Damage {
    let cursor = buf_view.cursor;

    let (display_width, next_col) = {
        let line = curr_line(config, text.rope(), buf_view);
        (line.display_width, line.next_insert_col(cursor.col))
    };

    if cursor.col < display_width {
        let char_idx = cursor_to_char_idx(config, buf_view, text.rope());
        let next_idx = coords_to_char_idx(
            config,
            text.rope(),
            buf_view,
            Coords::new(buf_view.cursor.row, next_col),
        );

        text.remove(char_idx..next_idx);
        patch_curr_line(config, text.rope(), buf_view);

        Damage::Line(buf_view.cursor.row)
    } else if buf_view.cursor.row < text.rope().len_lines().saturating_sub(1) {
        buf_view.cursor.row += 1;
        join_above(config, buf_view, text)
    } else {
        Damage::Intact
    }
}

fn join_above(config: &Config, buf_view: &mut BufferView, text: &mut impl MutBuffer) -> Damage {
    assert!(
        buf_view.cursor.row > 0,
        "already at top row, nothing to merge"
    );

    buf_view.cursor.col = 0;
    let row_above = buf_view.cursor.row - 1;
    let char_idx = cursor_to_char_idx(config, buf_view, text.rope());

    let mut prev_idx = char_idx - 1;
    if prev_idx > 0 && text.rope().char(prev_idx) == '\n' && text.rope().char(prev_idx - 1) == '\r'
    {
        prev_idx -= 1;
    }

    let line = buf_view
        .display_buf
        .ensure_line(config, text.rope(), row_above);
    buf_view.cursor.col = line.display_width;
    buf_view.target_col = buf_view.cursor.col;

    text.remove(prev_idx..char_idx);
    buf_view.display_buf.destroy_from(row_above);
    move_up::<InsertNav>(config, text.rope(), buf_view, 1);

    Damage::From(row_above)
}

fn patch_curr_line(config: &Config, rope: &Rope, buf_view: &mut BufferView) {
    let line_idx = buf_view.cursor.row;
    buf_view
        .display_buf
        .patch_range(config, rope, line_idx..line_idx + 1);
}
