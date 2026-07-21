use ropey::Rope;

use crate::components::{BufferView, Config, Coords, DisplayLineRef};

pub fn curr_line<'a>(
    config: &Config,
    rope: &Rope,
    buf_view: &'a mut BufferView,
) -> DisplayLineRef<'a> {
    buf_view
        .display_buf
        .ensure_line(config, rope, buf_view.cursor.row)
}

pub fn cursor_to_char_idx(config: &Config, buf_view: &mut BufferView, rope: &Rope) -> usize {
    coords_to_char_idx(config, rope, buf_view, buf_view.cursor)
}

pub fn coords_to_char_idx(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    coords: Coords,
) -> usize {
    if rope.len_chars() == 0 {
        return 0;
    }

    let line_idx = rope.line_to_char(coords.row);
    let display_line = curr_line(config, rope, buf_view);
    let col_idx = display_line.col_to_char_idx(coords.col);
    line_idx + col_idx
}

pub fn char_idx_to_coords(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    char_idx: usize,
) -> Coords {
    let line_idx = rope.char_to_line(char_idx);
    let start_idx = rope.line_to_char(line_idx);
    let line = buf_view.display_buf.ensure_line(config, rope, line_idx);
    let col_idx = line.char_idx_to_col(char_idx - start_idx);

    Coords {
        row: line_idx,
        col: col_idx,
    }
}

pub fn snap_coords(config: &Config, rope: &Rope, buf_view: &mut BufferView, coords: Coords) {
    let line = buf_view.display_buf.ensure_line(config, rope, coords.row);
    let col = line.snap_col(coords.col);

    buf_view.cursor.row = coords.row;
    buf_view.cursor.col = col;
    buf_view.target_col = col;
}
