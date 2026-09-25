use ropey::Rope;

use crate::components::{BufferView, Config, Coords, DisplayLineRef};

// Get the display line for the given row.
pub fn display_line<'a>(
    config: &Config,
    rope: &Rope,
    buf_view: &'a mut BufferView,
    row: usize,
) -> DisplayLineRef<'a> {
    buf_view.display_buf.ensure_line(config, rope, row)
}

// Get the current display line (that, the display line for the cursor row).
pub fn curr_line<'a>(
    config: &Config,
    rope: &Rope,
    buf_view: &'a mut BufferView,
) -> DisplayLineRef<'a> {
    display_line(config, rope, buf_view, buf_view.cursor.row)
}

pub fn cursor_to_char_idx(config: &Config, buf_view: &mut BufferView, rope: &Rope) -> usize {
    coords_to_char_idx(config, rope, buf_view, buf_view.cursor)
}

// Convert the given coords to a rope index, taking into account wide characters like
// control, non-visible, and wide chars (like emojis or tabs).
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
    let display_line = buf_view.display_buf.ensure_line(config, rope, coords.row);
    let col_idx = display_line.col_to_char_idx(coords.col);

    line_idx + col_idx
}

// Turn a rope index into the right column on the screen. Note: this function will
// return the "appropriate" column for normal mode navigation. That is:
//
//    - For a tab, this function will leave us at the rightmost on-screen column for the tab.
//    - For anything rendered wide (emojis, ctrl, zwj), this function will return the initial
//      column of the char. Again, what's expected in nav mode.
//
pub fn char_idx_to_coords(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    char_idx: usize,
) -> Coords {
    if rope.len_chars() == 0 {
        return Coords::new(0, 0);
    }

    let line_idx = rope.char_to_line(char_idx);
    let start_idx = rope.line_to_char(line_idx);
    let line = buf_view.display_buf.ensure_line(config, rope, line_idx);
    let col_idx = line.char_idx_to_col(char_idx - start_idx);
    let snapped_col = line.snap_col(col_idx);

    Coords {
        row: line_idx,
        col: snapped_col,
    }
}

// Get the column number of the next grapheme for the given coords, or the line's
// display width if no next grapheme. That would correspond to the line's carriage
// return (if the line is the the text's last).
pub fn next_col_or_display_width(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    coords: Coords,
) -> usize {
    let line = buf_view.display_buf.ensure_line(config, rope, coords.row);
    line.grapheme_at(coords.col)
        .map(|(_, span)| span.end)
        .unwrap_or(line.display_width)
}

// Are the given coords at the last column of their line?
pub fn is_last_col(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    coords: Coords,
) -> bool {
    let line = buf_view.display_buf.ensure_line(config, rope, coords.row);
    line.grapheme_at(coords.col).is_none()
}

// Get the display width of the row for the given coords. The display width is defined
// as one past the last display column.
pub fn display_width(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    coords: Coords,
) -> usize {
    let line = buf_view.display_buf.ensure_line(config, rope, coords.row);
    line.display_width
}

// Move the buf_view cursor to the given coords. This function will take care of adjusting
// the column to ensure that in doesn't end up in the middle of a wide char. The snapping
// happens according to Normal mode nav rules. That is:
//
//    - Tabs: put cursor in the tab's last screen column
//    - Wide chars (emoji, ctrl, zwj), go to first column of the char
//
// This function will leave the given buf_view cursor and taget_col properly set
pub fn snap_coords(config: &Config, rope: &Rope, buf_view: &mut BufferView, coords: Coords) {
    let line = buf_view.display_buf.ensure_line(config, rope, coords.row);
    let col = line.snap_col(coords.col);

    buf_view.cursor.row = coords.row;
    buf_view.cursor.col = col;
    buf_view.target_col = col;
}

#[cfg(test)]
mod test {
    use super::*;
    use std::assert_eq;

    #[test]
    fn test_eof_border() {
        let config = Config::default();
        let rope = Rope::from_str("a\n\n");
        let mut buf_view = BufferView::empty();
        let coords = Coords::new(2, 0);

        let char_idx = coords_to_char_idx(&config, &rope, &mut buf_view, coords);
        assert_eq!(char_idx, rope.len_chars());

        let new_coords = char_idx_to_coords(&config, &rope, &mut buf_view, char_idx);
        assert_eq!(new_coords, coords);
    }

    #[test]
    fn test_col_rope_conversions() {
        let config = Config::default();
        let rope = Rope::from_str("foo\t🏳️‍🌈 \tbar\n");
        let mut buf_view = BufferView::empty();

        for (orig_col, expected_col) in vec![
            (3, 7),
            (5, 7),
            (7, 7),
            (8, 8),
            (9, 8),
            (10, 10),
            (11, 15),
            (13, 15),
            (15, 15),
            (16, 16),
            (17, 17),
            (18, 18),
            (19, 19),
        ] {
            let orig_coords = Coords::new(0, orig_col);
            let char_idx = coords_to_char_idx(&config, &rope, &mut buf_view, orig_coords);
            let expected_coords = char_idx_to_coords(&config, &rope, &mut buf_view, char_idx);
            assert_eq!(expected_col, expected_coords.col);
        }
    }
}
