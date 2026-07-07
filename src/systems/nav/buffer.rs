use ropey::Rope;

use super::rules::NavRules;
use crate::components::{BufferView, Config};
use crate::rope;
use crate::systems::commons::{char_idx_to_coords, curr_line, cursor_to_char_idx};

fn apply_target_col<R: NavRules>(config: &Config, rope: &Rope, buf_view: &mut BufferView) {
    let target_col = buf_view.target_col;
    let line = curr_line(config, rope, buf_view);
    let max_col = R::max_allowed_width(&line);
    buf_view.cursor.col = R::snap_col(&line, target_col.min(max_col));
}

pub fn move_up<R: NavRules>(config: &Config, rope: &Rope, buf_view: &mut BufferView, rows: usize) {
    let rows = rows.min(buf_view.cursor.row);

    if rows != 0 {
        buf_view.cursor.row -= rows;
        apply_target_col::<R>(config, rope, buf_view);
    }
}

pub fn move_down<R: NavRules>(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    rows: usize,
) {
    let max_line_idx = rope.len_lines().saturating_sub(1);
    let rows = rows.min(max_line_idx.saturating_sub(buf_view.cursor.row));

    if rows != 0 {
        buf_view.cursor.row += rows;
        apply_target_col::<R>(config, rope, buf_view);
    }
}

pub fn move_left<R: NavRules>(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    cols: usize,
) {
    let cols = cols.min(buf_view.cursor.col);

    if cols != 0 {
        let mut prev_col = buf_view.cursor.col;
        let line = curr_line(config, rope, buf_view);

        for _ in 0..cols {
            prev_col = R::prev_col(&line, prev_col);
        }

        buf_view.cursor.col = prev_col;
        buf_view.target_col = prev_col;
    }
}

pub fn move_right<R: NavRules>(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    cols: usize,
) {
    let cursor_col = buf_view.cursor.col;
    let line = curr_line(config, rope, buf_view);

    let max_col = R::max_allowed_width(&line);
    let cols = cols.min(max_col.saturating_sub(cursor_col));

    if cols != 0 {
        let mut next_col = cursor_col;

        for _ in 0..cols {
            next_col = R::next_col(&line, next_col);
        }

        buf_view.cursor.col = next_col;
        buf_view.target_col = next_col;
    }
}

pub fn page_up<R: NavRules>(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    pages: usize,
    pg_size: usize,
) {
    buf_view.cursor.row = buf_view.cursor.row.saturating_sub(pages * pg_size);
    line_first_non_blank::<R>(config, rope, buf_view);
}

pub fn page_down<R: NavRules>(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    pages: usize,
    pg_size: usize,
) {
    let max_line_idx = rope.len_lines().saturating_sub(1);
    buf_view.cursor.row = buf_view
        .cursor
        .row
        .saturating_add(pages * pg_size)
        .min(max_line_idx);
    line_first_non_blank::<R>(config, rope, buf_view);
}

pub fn next_big_word(config: &Config, rope: &Rope, buf_view: &mut BufferView, reps: usize) {
    let mut char_idx = cursor_to_char_idx(config, buf_view, rope);

    for _ in 0..reps {
        char_idx = rope::next_big_word(rope, char_idx);
    }

    buf_view.cursor = char_idx_to_coords(config, rope, buf_view, char_idx);
    buf_view.target_col = buf_view.cursor.col;
}

pub fn next_sub_word(config: &Config, rope: &Rope, buf_view: &mut BufferView, reps: usize) {
    let mut char_idx = cursor_to_char_idx(config, buf_view, rope);

    for _ in 0..reps {
        char_idx = rope::next_sub_word(rope, char_idx);
    }

    buf_view.cursor = char_idx_to_coords(config, rope, buf_view, char_idx);
    buf_view.target_col = buf_view.cursor.col;
}

pub fn prev_big_word(config: &Config, rope: &Rope, buf_view: &mut BufferView, reps: usize) {
    let mut char_idx = cursor_to_char_idx(config, buf_view, rope);

    for _ in 0..reps {
        char_idx = rope::prev_big_word(rope, char_idx);
    }

    buf_view.cursor = char_idx_to_coords(config, rope, buf_view, char_idx);
    buf_view.target_col = buf_view.cursor.col;
}

pub fn prev_sub_word(config: &Config, rope: &Rope, buf_view: &mut BufferView, reps: usize) {
    let mut char_idx = cursor_to_char_idx(config, buf_view, rope);

    for _ in 0..reps {
        char_idx = rope::prev_sub_word(rope, char_idx);
    }

    buf_view.cursor = char_idx_to_coords(config, rope, buf_view, char_idx);
    buf_view.target_col = buf_view.cursor.col;
}

pub fn line_first_non_blank<R: NavRules>(config: &Config, rope: &Rope, buf_view: &mut BufferView) {
    let line = curr_line(config, rope, buf_view);

    buf_view.cursor.col = R::first_non_blank(&line);
    buf_view.target_col = buf_view.cursor.col;
}

pub fn start_of_line<R: NavRules>(config: &Config, rope: &Rope, buf_view: &mut BufferView) {
    let line = curr_line(config, rope, buf_view);
    let col = R::snap_col(&line, 0);

    buf_view.cursor.col = col;
    buf_view.target_col = col;
}

pub fn end_of_line<R: NavRules>(config: &Config, rope: &Rope, buf_view: &mut BufferView) {
    let line = curr_line(config, rope, buf_view);
    let col = R::max_allowed_width(&line);
    let col = R::snap_col(&line, col);

    buf_view.cursor.col = col;
    buf_view.target_col = col;
}

pub fn file_first_non_blank<R:NavRules>(config: &Config, rope: &Rope, buf_view: &mut BufferView) {
    let char_idx = rope::first_non_blank_char_idx(rope);
    let coords = char_idx_to_coords(config, rope, buf_view, char_idx);
    let line = buf_view.display_buf.ensure_line(config, rope, coords.row);
    let col = R::first_non_blank(&line);

    buf_view.cursor.row = coords.row;
    buf_view.cursor.col = col;
    buf_view.target_col = col
}

pub fn start_of_file<R: NavRules>(config: &Config, rope: &Rope, buf_view: &mut BufferView) {
    let line = buf_view.display_buf.ensure_line(config, rope, 0);
    let col = R::snap_col(&line, 0);

    buf_view.cursor.row = 0;
    buf_view.cursor.col = col;
    buf_view.target_col = col
}

pub fn end_of_file<R: NavRules>(config: &Config, rope: &Rope, buf_view: &mut BufferView) {
    let char_idx = rope.len_chars().saturating_sub(1);
    let coords = char_idx_to_coords(config, rope, buf_view, char_idx);
    let line = buf_view.display_buf.ensure_line(config, rope, coords.row);
    let col = R::max_allowed_width(&line);
    let col = R::snap_col(&line, col);

    buf_view.cursor.row = coords.row;
    buf_view.cursor.col = col;
    buf_view.target_col = col;
}
