use std::{borrow::Cow, ops::Range};

use regex::Regex;
use ropey::{Rope, RopeSlice};

pub fn norm(rope: &mut Rope) {
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
        RopeInfo {
            num_lines: rope.len_lines(),
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

pub fn slice_as_view(rope: &Rope, range: Range<usize>) -> RopeSlice<'_> {
    let line_idx_start = rope.line_to_char(range.start);
    let line_idx_end = rope.line_to_char(range.end);
    rope.slice(line_idx_start..line_idx_end)
}

pub fn slice_as_rope(rope: &Rope, range: Range<usize>) -> Rope {
    let slice = slice_as_view(rope, range);
    Rope::from(slice)
}

pub fn find_forward(
    rope: &Rope,
    regex: &Regex,
    start_line: usize,
    start_col: Option<usize>,
) -> Option<(usize, usize)> {
    let total_lines = rope.len_lines();

    if start_line >= total_lines {
        return None;
    }

    for current_line in start_line..total_lines {
        let line_start_char = rope.line_to_char(current_line);
        let line_slice = rope.line(current_line);

        let line_text = match line_slice.as_str() {
            Some(inline_str) => Cow::Borrowed(inline_str),
            None => Cow::Owned(line_slice.to_string()),
        };

        let search_offset = if current_line == start_line {
            start_col
                .and_then(|col| line_text.char_indices().nth(col))
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(0)
        } else {
            0
        };

        if let Some(m) = regex.find(&line_text[search_offset..]) {
            let match_start_byte = search_offset + m.start();
            let match_end_byte = search_offset + m.end();

            let absolute_start = line_start_char + line_text[..match_start_byte].chars().count();
            let absolute_end = line_start_char + line_text[..match_end_byte].chars().count();
            return Some((absolute_start, absolute_end));
        }
    }

    None
}

pub fn find_backward(
    rope: &Rope,
    regex: &Regex,
    start_line: usize,
    start_col: Option<usize>,
) -> Option<(usize, usize)> {
    let mut current_line = if start_line >= rope.len_lines() {
        rope.len_lines().saturating_sub(1)
    } else {
        start_line
    };

    loop {
        let line_start_char = rope.line_to_char(current_line);
        let line_slice = rope.line(current_line);

        let line_text = match line_slice.as_str() {
            Some(inline_str) => Cow::Borrowed(inline_str),
            None => Cow::Owned(line_slice.to_string()),
        };

        let search_end_byte = if current_line == start_line {
            start_col
                .and_then(|col| line_text.char_indices().nth(col))
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(line_text.len())
        } else {
            line_text.len()
        };

        let mut last_match_in_line = None;
        for m in regex.find_iter(&line_text[..search_end_byte]) {
            last_match_in_line = Some((m.start(), m.end()));
        }

        if let Some((local_start, local_end)) = last_match_in_line {
            let absolute_start = line_start_char + line_text[..local_start].chars().count();
            let absolute_end = line_start_char + line_text[..local_end].chars().count();
            return Some((absolute_start, absolute_end));
        }

        if current_line == 0 {
            break;
        }
        current_line -= 1;
    }

    None
}

#[cfg(test)]
mod find_tests {
    use super::*;
    use ropey::Rope;

    fn setup_test_rope() -> Rope {
        Rope::from_str(
            "The quick brown fox\n\
             jumps over the lazy dog\n\
             fox jumps again\n\
             hello world\n\
             🦀 rust emoji test 🦀\n\
             end of text",
        )
    }

    fn re(s: &str) -> Regex {
        Regex::new(s).unwrap()
    }

    #[test]
    fn test_find_forward_basic() {
        let rope = setup_test_rope();

        let res = find_forward(&rope, &re("fox"), 0, Some(0));
        assert!(res.is_some());
        let (start, end) = res.unwrap();
        assert_eq!(start, 16);
        assert_eq!(end, 19);
    }

    #[test]
    fn test_find_forward_respects_column() {
        let rope = setup_test_rope();

        let res = find_forward(&rope, &re("fox"), 0, Some(16));
        assert_eq!(res, Some((16, 19)));

        let res_past = find_forward(&rope, &re("fox"), 0, Some(17));
        assert!(res_past.is_some());
        let (start, end) = res_past.unwrap();

        // Verify it skipped line 0 and found the second instance
        let matched_str = rope.slice(start..end).to_string();
        assert_eq!(matched_str, "fox");
        assert!(start > 19);
    }

    #[test]
    fn test_find_forward_whole_line_range() {
        let rope = setup_test_rope();

        let res = find_forward(&rope, &re("The"), 0, None);
        assert_eq!(res, Some((0, 3)));
    }

    #[test]
    fn test_find_backward_basic() {
        let rope = setup_test_rope();

        // Start from line 3, search backward for "lazy" (which is on line 1)
        let res = find_backward(&rope, &re("lazy"), 3, Some(0));
        assert!(res.is_some());
        let (start, end) = res.unwrap();
        assert_eq!(rope.slice(start..end).to_string(), "lazy");
    }

    #[test]
    fn test_find_backward_same_line_cursor_bound() {
        let rope = setup_test_rope();

        // Line 2 is: "fox jumps again"
        // "jumps" starts at column 4 on that line.

        // If cursor is at column 10 (after "jumps"), searching backward should find it.
        let res_before = find_backward(&rope, &re("jumps"), 2, Some(10));
        assert!(res_before.is_some());

        // If cursor is at column 3 (before "jumps"), searching backward must skip it
        // and find the "jumps" back on Line 1.
        let res_after = find_backward(&rope, &re("jumps"), 2, Some(3));
        assert!(res_after.is_some());
        let (start, end) = res_after.unwrap();
        let line_of_match = rope.char_to_line(start);
        assert_eq!(line_of_match, 1);
        assert_eq!(rope.slice(start..end).to_string(), "jumps");
    }

    #[test]
    fn test_find_backward_whole_line_range() {
        let rope = setup_test_rope();

        let res = find_backward(&rope, &re("again"), 2, None);
        assert!(res.is_some());
        let (start, end) = res.unwrap();
        assert_eq!(rope.slice(start..end).to_string(), "again");
    }

    #[test]
    fn test_unicode_and_emojis() {
        let rope = setup_test_rope();

        // Line 4 contains multi-byte crab emojis: "🦀 rust emoji test 🦀"
        // Regex byte indexes will not align 1:1 with Rope char indexes.

        // Forward search check
        let res_fw = find_forward(&rope, &re("rust"), 4, Some(0));
        assert!(res_fw.is_some());
        let (start_fw, end_fw) = res_fw.unwrap();
        assert_eq!(rope.slice(start_fw..end_fw).to_string(), "rust");

        // Backward search check starting past the word "rust"
        let res_bw = find_backward(&rope, &re("r.st"), 4, Some(15));
        assert!(res_bw.is_some());
        let (start_bw, end_bw) = res_bw.unwrap();
        assert_eq!(rope.slice(start_bw..end_bw).to_string(), "rust");

        // Ensure character alignment is clean
        assert_eq!(start_fw, start_bw);
    }

    #[test]
    fn test_no_match_returns_none() {
        let rope = setup_test_rope();

        let res_fw = find_forward(&rope, &re("nonexistent_pattern"), 0, None);
        assert!(res_fw.is_none());

        let res_bw = find_backward(&rope, &re("nonexistent_pattern"), 5, None);
        assert!(res_bw.is_none());
    }

    #[test]
    fn test_out_of_bounds_graceful_handling() {
        let rope = setup_test_rope();

        // Passing a line index that doesn't exist shouldn't panic
        let res_fw = find_forward(&rope, &re("end"), 100, None);
        assert!(res_fw.is_none());

        let res_bw = find_backward(&rope, &re("The"), 100, None);
        // Should clamp to the last valid line and still find "The" at the top
        assert!(res_bw.is_some());
        assert_eq!(res_bw.unwrap(), (0, 3));
    }
}

fn is_sub_word_char(c: char) -> bool {
    c == '_' || (!c.is_whitespace() && !c.is_ascii_punctuation())
}

pub fn next_big_word(rope: &Rope, char_idx: usize) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    if max_idx == 0 {
        return 0;
    }

    let mut char_idx = char_idx.clamp(0, max_idx);
    let mut c = rope.char(char_idx);

    while char_idx < max_idx && !c.is_whitespace() {
        char_idx += 1;
        c = rope.char(char_idx);
    }

    while char_idx < max_idx && c.is_whitespace() {
        char_idx += 1;
        c = rope.char(char_idx);
    }

    char_idx.min(rope.len_chars().saturating_sub(1))
}

pub fn next_sub_word(rope: &Rope, char_idx: usize) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    if max_idx == 0 {
        return 0;
    }

    let mut char_idx = char_idx.clamp(0, max_idx);
    let mut c = rope.char(char_idx);

    if is_sub_word_char(c) {
        while char_idx < max_idx && is_sub_word_char(c) {
            char_idx += 1;
            c = rope.char(char_idx);
        }
    } else if c.is_ascii_punctuation() {
        while char_idx < max_idx && c.is_ascii_punctuation() {
            char_idx += 1;
            c = rope.char(char_idx);
        }
    }

    while char_idx < max_idx && c.is_whitespace() {
        char_idx += 1;
        c = rope.char(char_idx);
    }

    char_idx.min(rope.len_chars().saturating_sub(1))
}

pub fn prev_big_word(rope: &Rope, char_idx: usize) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    if max_idx == 0 {
        return 0;
    }

    let mut char_idx = char_idx.clamp(0, max_idx);
    let mut c = rope.char(char_idx.saturating_sub(1));

    while char_idx != 0 && c.is_whitespace() {
        char_idx -= 1;
        c = rope.char(char_idx.saturating_sub(1));
    }

    while char_idx != 0 && !c.is_whitespace() {
        char_idx -= 1;
        c = rope.char(char_idx.saturating_sub(1));
    }

    char_idx.min(rope.len_chars().saturating_sub(1))
}

pub fn prev_sub_word(rope: &Rope, char_idx: usize) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    if max_idx == 0 {
        return 0;
    }

    let mut char_idx = char_idx.clamp(0, max_idx);
    let mut c = rope.char(char_idx.saturating_sub(1));

    while char_idx != 0 && c.is_whitespace() {
        char_idx -= 1;
        c = rope.char(char_idx.saturating_sub(1));
    }

    if is_sub_word_char(c) {
        while char_idx != 0 && is_sub_word_char(c) {
            char_idx -= 1;
            c = rope.char(char_idx.saturating_sub(1));
        }
    } else if c.is_ascii_punctuation() {
        while char_idx != 0 && c.is_ascii_punctuation() {
            char_idx -= 1;
            c = rope.char(char_idx.saturating_sub(1));
        }
    }

    char_idx.min(rope.len_chars().saturating_sub(1))
}

#[cfg(test)]
mod word_ws_tests {
    use super::*;
    use ropey::Rope;

    macro_rules! check_jump {
        ($func:ident, $text:expr, $char_idx: expr, Expected => $line:expr) => {
            let rope = Rope::from_str($text);
            let res = $func(&rope, $char_idx);
            assert_eq!(res, $line, "Failed on text: {:?}", $text);
        };
    }

    #[test]
    fn test_next_word_empty() {
        check_jump!(next_big_word, "", 0, Expected => 0);
        check_jump!(next_big_word, "", 20, Expected => 0);
    }

    #[test]
    fn test_next_word_spaces() {
        check_jump!(next_big_word, "    ", 0, Expected => 3);
    }

    #[test]
    fn test_next_word_standard_jump() {
        // Cursor on 'm' of 'mut' -> should jump to 'f' of 'foo'
        check_jump!(next_big_word, "mut foo", 0, Expected => 4);

        // Cursor on 'u' of 'mut' -> should still jump to 'f' of 'foo'
        check_jump!(next_big_word, "mut foo", 1, Expected => 4);

        // Cursor on 'f' of 'foo' -> should jump to last index
        check_jump!(next_big_word, "mut foo", 4, Expected => 6);
    }

    #[test]
    fn test_next_word_multiple_spaces() {
        // Should cross multiple spaces seamlessly
        check_jump!(next_big_word, "let    value", 0, Expected => 7);
    }

    #[test]
    fn test_next_word_crosses_newlines() {
        // Should drop down to the next line if a word ends near a newline
        let text = "first\nsecond";
        check_jump!(next_big_word, text, 0, Expected => 6);

        // From a blank line, it should jump to the first word below it
        let blank_line_text = "first\n\n\n  second";
        check_jump!(next_big_word, blank_line_text, 5, Expected => 10);
    }

    #[test]
    fn test_next_word_boundaries() {
        // Last word in the file has no next word -> returns None
        check_jump!(next_big_word, "tail", 0, Expected => 3);
        check_jump!(next_big_word, "tail   ", 0, Expected => 6);

        // Out of bounds inputs
        check_jump!(next_big_word, "hello", 5, Expected => 4);
    }

    #[test]
    fn test_next_word_punct_and_emojis() {
        check_jump!(next_sub_word, "   -A-  !", 4, Expected => 5);
        check_jump!(next_big_word, "a_b a", 0, Expected => 4);
        check_jump!(next_sub_word, "a_b-- a", 0, Expected => 3);
        check_jump!(next_big_word, "a🧑‍🧑‍🧒‍🧒b a", 0, Expected => 10);
        check_jump!(next_sub_word, "a🧑‍🧑‍🧒‍🧒b a", 0, Expected => 10);
    }

    #[test]
    fn test_prev_word_empty() {
        check_jump!(prev_big_word, "", 0, Expected => 0);
        check_jump!(prev_big_word, "", 20, Expected => 0);
    }

    #[test]
    fn test_prev_word_spaces() {
        check_jump!(prev_big_word, "    ", 3, Expected => 0);
    }

    #[test]
    fn test_prev_word_standard_jump() {
        // Cursor on 'f' of 'foo' -> should jump back to 'm' of 'mut'
        check_jump!(prev_big_word, "mut foo", 4, Expected => 0);

        // Cursor in the middle of 'foo' -> should jump back to 'f'
        check_jump!(prev_big_word, "mut foo", 5, Expected => 4);
    }

    #[test]
    fn test_prev_word_multiple_spaces() {
        // Should cross multiple spaces backwards seamlessly
        check_jump!(prev_big_word, "first let    value", 13, Expected => 6);

        // Should go to the beginning of the current word
        check_jump!(prev_big_word, "first let    value", 7, Expected => 6);
    }

    #[test]
    fn test_prev_word_lands_at_start_of_file() {
        // Your specific unwrap_or edge-case: jumping to the very first word of a file
        check_jump!(prev_big_word, "first_word  ", 11, Expected => 0);
        check_jump!(prev_big_word, "first_word", 5, Expected => 0);
    }

    #[test]
    fn test_prev_word_crosses_newlines_backward() {
        // Should move up a line if spaces/newlines are encountered backwards
        let text = "first\nsecond";
        check_jump!(prev_big_word, text, 6, Expected => 0);

        let multi_line = "first\n\n\n  second";
        check_jump!(prev_big_word, multi_line, 10, Expected => 0);

        let multi_line_1 = "first\n\n\n  second third";
        check_jump!(prev_big_word, multi_line_1, 17, Expected => 10);
    }

    #[test]
    fn test_prev_word_punct_and_emojis() {
        check_jump!(prev_big_word, "a_b a", 4, Expected => 0);
        check_jump!(prev_sub_word, "a_b-- a", 6, Expected => 3);
        check_jump!(prev_big_word, "a🧑‍🧑‍🧒‍🧒b a", 10, Expected => 0);
        check_jump!(prev_sub_word, "a🧑‍🧑‍🧒‍🧒b a", 10, Expected => 0);
    }
}
