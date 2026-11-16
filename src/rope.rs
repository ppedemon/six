use std::ops::Range;

use ropey::{Rope, RopeSlice};

mod char;
mod search;
mod word;

pub use char::{find_char_backward, find_char_forward, till_char_backward, till_char_forward};
pub use search::{find_backward, find_forward};
pub use word::{
    end_big_word, end_sub_word, is_sub_word_char, next_big_word, next_sub_word, prev_big_word,
    prev_sub_word,
};

pub fn remove_trailing_newline(rope: &mut Rope) {
    let len_chars = rope.len_chars();
    if len_chars > 0 && rope.char(len_chars - 1) == '\n' {
        rope.remove(len_chars - 1..len_chars);
    }
}

pub struct RopeInfo {
    pub num_lines: usize,
    pub num_bytes: usize,
}

pub fn info_slice(rope: RopeSlice<'_>) -> RopeInfo {
    if rope.len_chars() == 0 {
        RopeInfo {
            num_lines: 0,
            num_bytes: 0,
        }
    } else {
        // Do norm here without mutating the rope
        let mut num_lines = rope.len_lines();
        if rope.char(rope.len_chars() - 1) == '\n' {
            num_lines -= 1;
        }
        RopeInfo {
            num_lines,
            num_bytes: rope.len_bytes(),
        }
    }
}

// Duplicating info_slice for the Rope type to avoid doing rope.slice(..), which is log(N)
pub fn info(rope: &Rope) -> RopeInfo {
    if rope.len_chars() == 0 {
        RopeInfo {
            num_lines: 0,
            num_bytes: 0,
        }
    } else {
        RopeInfo {
            num_lines: rope.len_lines(),
            num_bytes: rope.len_bytes(),
        }
    }
}

pub fn first_non_blank_char_idx(rope: &Rope) -> usize {
    rope.chars().position(|c| !c.is_whitespace()).unwrap_or(0)
}

pub fn slice_as_view(rope: &Rope, range: Range<usize>) -> RopeSlice<'_> {
    let line_idx_start = rope.line_to_char(range.start);
    let line_idx_end = rope.line_to_char(range.end);
    rope.slice(line_idx_start..line_idx_end)
}

pub fn slice_as_rope(rope: &Rope, range: Range<usize>) -> Rope {
    let slice = slice_as_view(rope, range);
    Rope::from(slice)
}
