use std::borrow::Cow;
use std::ops::Range;

use regex::Regex;
use ropey::Rope;

pub struct SearchResult {
    pub wrap: bool,
    pub start: usize,
    pub end: usize,
}

impl SearchResult {
    fn new(wrap: bool, start: usize, end: usize) -> Self {
        Self { wrap, start, end }
    }

    fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
}

// Finds the first regex match at or after `start_idx`.
//
// Returns the match's half-open character range, or `None` if no match is found,
// the search range is empty, or `start_idx` is outside the rope.
pub fn find_forward(rope: &Rope, start_idx: usize, regex: &Regex) -> Option<SearchResult> {
    range_find_forward(rope, start_idx..rope.len_chars(), regex)
}

pub fn wrap_find_forward(rope: &Rope, start_idx: usize, regex: &Regex) -> Option<SearchResult> {
    find_forward(rope, start_idx, regex)
        .or(range_find_forward(rope, 0..start_idx, regex).map(SearchResult::wrap))
}

// Finds the first regex match in the given character range, searching forward.
//
// `range` uses Ropey character indices and follows Rust's half-open range
// convention: `range.start` is included and `range.end` is excluded.
//
// Returns the match's half-open character range, or `None` if the range is empty,
// reversed, out of bounds, or contains no match.
pub fn range_find_forward(rope: &Rope, range: Range<usize>, regex: &Regex) -> Option<SearchResult> {
    if range.start >= range.end || range.end > rope.len_chars() {
        return None;
    }

    let slice = rope.slice(range.clone());
    let haystack = match slice.as_str() {
        Some(s) => Cow::Borrowed(s),
        None => Cow::Owned(slice.to_string()),
    };

    if let Some(m) = regex.find(&haystack) {
        let match_start = range.start + slice.byte_to_char(m.start());
        let match_end = range.start + slice.byte_to_char(m.end());
        return Some(SearchResult::new(false, match_start, match_end));
    }

    None
}

// Finds the last regex match contained in `[0, before_idx)`.
//
// The search covers `[0, before_idx)`, so `before_idx` is an exclusive boundary.
// Passing `rope.len_chars()` searches the whole rope.
//
// Returns the match's half-open character range, or `None` if no match is found
// or `before_idx` is zero or outside the rope.
pub fn find_backward(rope: &Rope, before_idx: usize, regex: &Regex) -> Option<SearchResult> {
    range_find_backward(rope, 0..before_idx, regex)
}

pub fn wrap_find_backward(rope: &Rope, before_idx: usize, regex: &Regex) -> Option<SearchResult> {
    find_backward(rope, before_idx, regex).or(range_find_backward(
        rope,
        before_idx..rope.len_chars(),
        regex,
    )
    .map(SearchResult::wrap))
}

// Finds the last regex match in the given character range, searching backward.
//
// `range` uses Ropey character indices and follows Rust's half-open range
// convention: `range.start` is included and `range.end` is excluded.
//
// Returns the match's half-open character range, or `None` if the range is empty,
// reversed, out of bounds, or contains no match.
pub fn range_find_backward(
    rope: &Rope,
    range: Range<usize>,
    regex: &Regex,
) -> Option<SearchResult> {
    if range.start >= range.end || range.end > rope.len_chars() {
        return None;
    }

    let slice = rope.slice(range.clone());
    let haystack = match slice.as_str() {
        Some(s) => Cow::Borrowed(s),
        None => Cow::Owned(slice.to_string()),
    };

    let mut last_match = None;
    for m in regex.find_iter(&haystack) {
        last_match = Some(m);
    }

    if let Some(m) = last_match {
        let match_start = range.start + slice.byte_to_char(m.start());
        let match_end = range.start + slice.byte_to_char(m.end());
        return Some(SearchResult::new(false, match_start, match_end));
    }

    None
}

#[cfg(test)]
mod tests {
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

    fn bounds(result: Option<SearchResult>) -> Option<(usize, usize)> {
        result.map(|result| (result.start, result.end))
    }

    #[test]
    fn test_find_forward_basic() {
        let rope = setup_test_rope();

        let res = find_forward(&rope, 0, &re("fox"));
        assert!(res.is_some());
        let result = res.unwrap();
        let (start, end) = (result.start, result.end);
        assert_eq!(start, 16);
        assert_eq!(end, 19);
    }

    #[test]
    fn test_find_forward_respects_column() {
        let rope = setup_test_rope();

        let res = find_forward(&rope, 16, &re("fox"));
        assert_eq!(bounds(res), Some((16, 19)));

        let res_past = find_forward(&rope, 17, &re("fox"));
        assert!(res_past.is_some());
        let result = res_past.unwrap();
        let (start, end) = (result.start, result.end);

        // Verify it skipped line 0 and found the second instance
        let matched_str = rope.slice(start..end).to_string();
        assert_eq!(matched_str, "fox");
        assert!(start > 19);
    }

    #[test]
    fn test_find_forward_whole_line_range() {
        let rope = setup_test_rope();

        let res = find_forward(&rope, 0, &re("The"));
        assert_eq!(bounds(res), Some((0, 3)));
    }

    #[test]
    fn test_range_find_forward_respects_both_bounds() {
        let rope = Rope::from_str("one two one\none two");

        assert_eq!(
            bounds(range_find_forward(&rope, 4..7, &re("two"))),
            Some((4, 7))
        );
        assert_eq!(bounds(range_find_forward(&rope, 5..7, &re("two"))), None);
        assert_eq!(
            bounds(range_find_forward(&rope, 8..16, &re("one"))),
            Some((8, 11))
        );
    }

    #[test]
    fn test_forward_search_finds_matches_spanning_multiple_lines() {
        let rope = Rope::from_str("north\nsouth\neast\nnorth\nsouth\neast");
        let regex = re("north\\nsouth\\neast");

        assert_eq!(
            bounds(range_find_forward(&rope, 0..16, &regex)),
            Some((0, 16))
        );
        assert_eq!(bounds(find_forward(&rope, 1, &regex)), Some((17, 33)));
        assert_eq!(bounds(range_find_forward(&rope, 1..16, &regex)), None);
    }

    #[test]
    fn test_wrap_find_forward_reports_wrapped_and_unwrapped_matches() {
        let rope = Rope::from_str("first\nsecond");

        let unwrapped = wrap_find_forward(&rope, 0, &re("first")).unwrap();
        assert_eq!((unwrapped.start, unwrapped.end), (0, 5));
        assert!(!unwrapped.wrap);

        let wrapped = wrap_find_forward(&rope, 6, &re("first")).unwrap();
        assert_eq!((wrapped.start, wrapped.end), (0, 5));
        assert!(wrapped.wrap);

        assert!(wrap_find_forward(&rope, 0, &re("missing")).is_none());
    }

    #[test]
    fn test_range_find_forward_rejects_empty_reversed_and_out_of_bounds_ranges() {
        let rope = Rope::from_str("one two");

        assert_eq!(bounds(range_find_forward(&rope, 2..2, &re("one"))), None);
        assert_eq!(bounds(range_find_forward(&rope, 5..3, &re("one"))), None);
        assert_eq!(bounds(range_find_forward(&rope, 0..8, &re("one"))), None);
    }

    #[test]
    fn test_find_backward_basic() {
        let rope = setup_test_rope();

        // Start from line 3, search backward for "lazy" (which is on line 1)
        let res = find_backward(&rope, 60, &re("lazy"));
        assert!(res.is_some());
        let result = res.unwrap();
        let (start, end) = (result.start, result.end);
        assert_eq!(rope.slice(start..end).to_string(), "lazy");
    }

    #[test]
    fn test_find_backward_same_line_cursor_bound() {
        let rope = setup_test_rope();

        // Line 2 is: "fox jumps again"
        // "jumps" starts at column 4 on that line.

        // If cursor is at column 10 (after "jumps"), searching backward should find it.
        let res_before = find_backward(&rope, 54, &re("jumps"));
        assert!(res_before.is_some());

        // If cursor is at column 3 (before "jumps"), searching backward must skip it
        // and find the "jumps" back on Line 1.
        let res_after = find_backward(&rope, 47, &re("jumps"));
        assert!(res_after.is_some());
        let result = res_after.unwrap();
        let (start, end) = (result.start, result.end);
        let line_of_match = rope.char_to_line(start);
        assert_eq!(line_of_match, 1);
        assert_eq!(rope.slice(start..end).to_string(), "jumps");
    }

    #[test]
    fn test_find_backward_whole_line_range() {
        let rope = setup_test_rope();

        let res = find_backward(&rope, 60, &re("again"));
        assert!(res.is_some());
        let result = res.unwrap();
        let (start, end) = (result.start, result.end);
        assert_eq!(rope.slice(start..end).to_string(), "again");
    }

    #[test]
    fn test_range_find_backward_respects_both_bounds() {
        let rope = Rope::from_str("one two one\none two");

        assert_eq!(
            bounds(range_find_backward(&rope, 0..7, &re("two"))),
            Some((4, 7))
        );
        assert_eq!(
            bounds(range_find_backward(&rope, 0..8, &re("one|two"))),
            Some((4, 7))
        );
        assert_eq!(bounds(range_find_backward(&rope, 5..8, &re("two"))), None);
        assert_eq!(
            bounds(range_find_backward(&rope, 5..11, &re("two|one"))),
            Some((8, 11))
        );
        assert_eq!(
            bounds(range_find_backward(&rope, 4..16, &re("one|two"))),
            Some((12, 15))
        );
        assert_eq!(
            bounds(range_find_backward(&rope, 5..13, &re("one|two"))),
            Some((8, 11))
        );
        assert_eq!(
            bounds(range_find_backward(&rope, 12..19, &re("one|two"))),
            Some((16, 19))
        );
    }

    #[test]
    fn test_backward_search_finds_last_match_spanning_multiple_lines() {
        let rope = Rope::from_str("north\nsouth\neast\nnorth\nsouth\neast");
        let regex = re("north\\nsouth\\neast");

        assert_eq!(
            bounds(range_find_backward(&rope, 0..rope.len_chars(), &regex)),
            Some((17, 33))
        );
        assert_eq!(bounds(find_backward(&rope, 17, &regex)), Some((0, 16)));
        assert_eq!(bounds(range_find_backward(&rope, 1..16, &regex)), None);
    }

    #[test]
    fn test_wrap_find_backward_reports_wrapped_and_unwrapped_matches() {
        let rope = Rope::from_str("first\nsecond");

        let unwrapped = wrap_find_backward(&rope, rope.len_chars(), &re("second")).unwrap();
        assert_eq!((unwrapped.start, unwrapped.end), (6, 12));
        assert!(!unwrapped.wrap);

        let wrapped = wrap_find_backward(&rope, 6, &re("second")).unwrap();
        assert_eq!((wrapped.start, wrapped.end), (6, 12));
        assert!(wrapped.wrap);

        assert!(wrap_find_backward(&rope, 0, &re("missing")).is_none());
    }

    #[test]
    fn test_range_find_backward_rejects_empty_reversed_and_out_of_bounds_ranges() {
        let rope = Rope::from_str("one two");

        assert_eq!(bounds(range_find_backward(&rope, 2..2, &re("one"))), None);
        assert_eq!(bounds(range_find_backward(&rope, 5..3, &re("one"))), None);
        assert_eq!(bounds(range_find_backward(&rope, 0..8, &re("one"))), None);
    }

    #[test]
    fn test_find_backward_accepts_end_of_rope_as_exclusive_boundary() {
        let rope = Rope::from_str("one two");

        assert_eq!(
            bounds(find_backward(&rope, rope.len_chars(), &re("two"))),
            Some((4, 7))
        );
        assert_eq!(bounds(find_backward(&rope, 0, &re("one"))), None);
    }

    #[test]
    fn test_unicode_and_emojis() {
        let rope = setup_test_rope();

        // Line 4 contains multi-byte crab emojis: "🦀 rust emoji test 🦀"
        // Regex byte indexes will not align 1:1 with Rope char indexes.

        // Forward search check
        let start_idx = rope.line_to_char(4);
        let res_fw = find_forward(&rope, start_idx, &re("rust"));
        assert!(res_fw.is_some());
        let result_fw = res_fw.unwrap();
        let (start_fw, end_fw) = (result_fw.start, result_fw.end);
        assert_eq!(rope.slice(start_fw..end_fw).to_string(), "rust");

        // Backward search check starting past the word "rust"
        let res_bw = find_backward(&rope, 87, &re("r.st"));
        assert!(res_bw.is_some());
        let result_bw = res_bw.unwrap();
        let (start_bw, end_bw) = (result_bw.start, result_bw.end);
        assert_eq!(rope.slice(start_bw..end_bw).to_string(), "rust");

        // Ensure character alignment is clean
        assert_eq!(start_fw, start_bw);
    }

    #[test]
    fn test_range_search_on_multi_chunk_line() {
        let prefix = "x".repeat(1500);
        let suffix = "y".repeat(1500);
        let rope = Rope::from_str(&format!("{prefix}needle{suffix}"));
        let needle_start = prefix.chars().count();
        let needle_end = needle_start + "needle".chars().count();
        let range = 1000..2000;

        assert_eq!(
            bounds(range_find_forward(&rope, range.clone(), &re("needle"))),
            Some((needle_start, needle_end))
        );
        assert_eq!(
            bounds(range_find_backward(&rope, range, &re("needle"))),
            Some((needle_start, needle_end))
        );
    }

    #[test]
    fn test_no_match_returns_none() {
        let rope = setup_test_rope();

        let res_fw = find_forward(&rope, 0, &re("nonexistent_pattern"));
        assert!(res_fw.is_none());

        let res_bw = find_backward(&rope, 103, &re("nonexistent_pattern"));
        assert!(res_bw.is_none());
    }

    #[test]
    fn test_out_of_bounds_graceful_handling() {
        let rope = setup_test_rope();

        // Passing a character index beyond the rope shouldn't panic.
        let res_fw = find_forward(&rope, 1000, &re("end"));
        assert!(res_fw.is_none());

        let res_bw = find_backward(&rope, 1000, &re("The"));
        // An out-of-range boundary should return None without panicking.
        assert!(res_bw.is_none());
    }
}
