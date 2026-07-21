use std::fmt::Write;
use std::ops::{Deref, Range};

use ropey::{Rope, RopeSlice};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::components::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Default for Span {
    fn default() -> Self {
        Self { start: 0, end: 0 }
    }
}

pub struct DisplayLine {
    pub text: String,
    pub byte_offsets: Vec<usize>,
    pub rope_indices: Vec<usize>,
    pub display_cols: Vec<usize>,
    pub display_width: usize,
}

impl DisplayLine {
    pub fn new(rope_slice: RopeSlice, tab_width: usize) -> Self {
        let line = rope_slice.to_string();
        let line = line.trim_end_matches(['\n', '\r']);

        let mut text = String::with_capacity(line.len());
        let mut byte_offsets = Vec::with_capacity(line.len());
        let mut rope_indices = Vec::with_capacity(line.len());
        let mut display_cols = Vec::with_capacity(line.len());

        let mut display_width = 0;
        let mut rope_idx = 0;

        let mut render_buf = String::with_capacity(8);

        for g in line.graphemes(true) {
            let g_char_len = g.chars().count();

            if Self::is_ctrl(g) {
                Self::ctrl_to_str(g.as_bytes()[0], &mut render_buf);
                for c in render_buf.chars() {
                    byte_offsets.push(text.len());
                    rope_indices.push(rope_idx);
                    display_cols.push(display_width);
                    text.push(c);
                    display_width += 1;
                }
            } else if g == "\t" {
                let width = tab_width - (display_width % tab_width);
                for _ in 0..width {
                    byte_offsets.push(text.len());
                    rope_indices.push(rope_idx);
                    display_cols.push(display_width);
                    text.push(' ');
                    display_width += 1;
                }
            } else {
                let width = g.width();
                if width > 0 {
                    byte_offsets.push(text.len());
                    rope_indices.push(rope_idx);
                    display_cols.push(display_width);
                    text.push_str(g);
                    display_width += width;
                } else {
                    Self::zwj_to_str(g, &mut render_buf);
                    for c in render_buf.chars() {
                        byte_offsets.push(text.len());
                        rope_indices.push(rope_idx);
                        display_cols.push(display_width);
                        text.push(c);
                        display_width += 1;
                    }
                }
            }
            rope_idx += g_char_len;
        }

        byte_offsets.push(text.len());
        rope_indices.push(rope_idx);
        display_cols.push(display_width);

        // panic!(
        //     "{:?}\n{:?}\n{:?}\n{:?}",
        //     text.bytes().collect::<Vec<_>>(),
        //     byte_offsets,
        //     rope_indices,
        //     display_cols
        // );

        Self {
            text,
            byte_offsets,
            rope_indices,
            display_cols,
            display_width,
        }
    }

    fn is_ctrl(g: &str) -> bool {
        g.len() == 1 && g != "\t" && g.chars().next().is_some_and(|c| c.is_control())
    }

    fn ctrl_to_str(code: u8, result: &mut String) {
        result.clear();
        if code < 0x20 {
            result.push('^');
            let c = char::from_u32(('@' as u32) + code as u32).unwrap_or('?');
            result.push(c);
        } else if code == 0x7f {
            result.push_str("^?")
        } else {
            write!(result, "<{:02X}>", code).unwrap();
        }
    }

    fn zwj_to_str(g: &str, result: &mut String) {
        result.clear();
        for c in g.chars() {
            write!(result, "<{:x}>", c as u32).unwrap();
        }
    }

    pub fn col_to_char_idx(&self, col: usize) -> usize {
        let idx = match self.display_cols.binary_search(&col) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        self.rope_indices[idx]
    }

    pub fn char_idx_to_col(&self, rope_idx: usize) -> usize {
        let idx = match self.rope_indices.binary_search(&rope_idx) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        self.display_cols[idx]
    }

    pub fn grapheme_at(&self, col: usize) -> Option<(&str, Span)> {
        let pos = match self.display_cols.binary_search(&col) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };

        if pos + 1 >= self.display_cols.len() {
            None
        } else {
            let mut start = pos;
            while start > 0 && self.rope_indices[start - 1] == self.rope_indices[pos] {
                start -= 1;
            }

            let mut end = pos;
            while end + 1 < self.rope_indices.len()
                && self.rope_indices[end + 1] == self.rope_indices[pos]
            {
                end += 1;
            }

            let g = &self.text[self.byte_offsets[start]..self.byte_offsets[end + 1]];
            let span = Span {
                start: self.display_cols[start],
                end: self.display_cols[end + 1],
            };
            Some((g, span))
        }
    }

    pub fn graphemes(&self) -> GraphemeIterator<'_> {
        self.graphemes_between(0, self.display_width)
    }

    // Iterate graphemes forward, in the interval [start_col, end_col)
    pub fn graphemes_between(&self, start_col: usize, end_col: usize) -> GraphemeIterator<'_> {
        GraphemeIterator {
            line: self,
            start_col,
            end_col,
        }
    }

    pub fn rev_graphemes(&self) -> RevGraphemeIterator<'_> {
        self.rev_graphemes_between(self.display_width, 0)
    }

    // Iterate graphemes backwards, in the interval (start_col, end_col]
    pub fn rev_graphemes_between(
        &self,
        start_col: usize,
        end_col: usize,
    ) -> RevGraphemeIterator<'_> {
        // NOTE! Since the start is open, we must ignore the grapheme (if any) at start_col.
        // Therefore, if we are insinde the line, we get the span for such grapheme and set
        // start_col to its first column. The iterator will start from the column before,
        // that is, the last column of the prev span.
        let start_col = self
            .grapheme_at(start_col)
            .map_or(self.display_width, |(_, span)| span.start);

        RevGraphemeIterator {
            line: self,
            start_col,
            end_col,
        }
    }

    pub fn prev_col(&self, col: usize) -> usize {
        let prev = match self.grapheme_at(col) {
            None => self.grapheme_at(self.display_width.saturating_sub(1)),
            Some((_, span)) => self.grapheme_at(span.start.saturating_sub(1)),
        };

        match prev {
            None => 0,
            Some((g, span)) => {
                if Self::is_tab(g) {
                    span.end - 1
                } else {
                    span.start
                }
            }
        }
    }

    pub fn next_col(&self, col: usize) -> usize {
        let curr = self.grapheme_at(col);

        let next = match curr {
            None => self.grapheme_at(self.display_width.saturating_sub(1)),
            Some((_, ref span)) => self.grapheme_at(span.end),
        };

        match next.or(curr) {
            None => 0,
            Some((g, span)) => {
                if Self::is_tab(g) {
                    span.end - 1
                } else {
                    span.start
                }
            }
        }
    }

    pub fn snap_col(&self, col: usize) -> usize {
        match self.grapheme_at(col) {
            None => col,
            Some((g, span)) => {
                if Self::is_tab(g) {
                    span.end - 1
                } else {
                    span.start
                }
            }
        }
    }

    pub fn first_non_blank(&self) -> usize {
        let mut last_grapheme = None;

        for (g, span) in self.graphemes() {
            let is_whitespace = g.chars().next().is_none_or(|c| c.is_whitespace());
            if !is_whitespace && !Self::is_tab(g) {
                return span.start;
            }
            last_grapheme = Some((g, span));
        }

        match last_grapheme {
            None => 0,
            Some((g, span)) => {
                if Self::is_tab(g) {
                    span.end - 1
                } else {
                    span.start
                }
            }
        }
    }

    // Next and previous column calculation works differently if we are in insert mode:
    //
    //  1. For tabs, in insert mode we shall move to the beginning of the tab filler, not to the end as in normal mode
    //  2. If we are in the last column, in insert mode we can move one position to the left to append text to the line
    //  3. For tabs, snapping should go to the beginning of the tab filler, not to the end as in normal mode
    //  4. Last non-blank insert column must be one past the last non-blank column in insert mode

    pub fn prev_insert_col(&self, col: usize) -> usize {
        let prev = match self.grapheme_at(col) {
            None => self.grapheme_at(self.display_width.saturating_sub(1)),
            Some((_, span)) => self.grapheme_at(span.start.saturating_sub(1)),
        };

        match prev {
            None => 0,
            Some((_, span)) => span.start,
        }
    }

    pub fn next_insert_col(&self, col: usize) -> usize {
        let next = match self.grapheme_at(col) {
            None => return self.display_width,
            Some((_, ref span)) => self.grapheme_at(span.end),
        };

        match next {
            None => self.display_width,
            Some((_, span)) => span.start,
        }
    }

    pub fn snap_insert_col(&self, col: usize) -> usize {
        match self.grapheme_at(col) {
            None => col,
            Some((_, span)) => span.start,
        }
    }

    pub fn first_insert_non_blank(&self) -> usize {
        let mut last_grapheme = None;

        for (g, span) in self.graphemes() {
            let is_whitespace = g.chars().next().is_none_or(|c| c.is_whitespace());
            if !is_whitespace && !Self::is_tab(g) {
                return span.start;
            }
            last_grapheme = Some((g, span));
        }

        match last_grapheme {
            None => 0,
            Some((_, span)) => span.start,
        }
    }

    fn is_tab(g: &str) -> bool {
        g.len() > 1 && g.chars().all(|c| c == ' ')
    }
}

pub struct GraphemeIterator<'a> {
    line: &'a DisplayLine,
    start_col: usize,
    end_col: usize,
}

impl<'a> Iterator for GraphemeIterator<'a> {
    type Item = (&'a str, Span);
    fn next(&mut self) -> Option<Self::Item> {
        if self.start_col >= self.end_col {
            None
        } else {
            let next_grapheme = self.line.grapheme_at(self.start_col);
            match &next_grapheme {
                Some((_, span)) => self.start_col = span.end,
                None => {}
            };
            next_grapheme
        }
    }
}

pub struct RevGraphemeIterator<'a> {
    line: &'a DisplayLine,
    start_col: usize,
    end_col: usize,
}

impl<'a> Iterator for RevGraphemeIterator<'a> {
    type Item = (&'a str, Span);
    fn next(&mut self) -> Option<Self::Item> {
        if self.start_col <= self.end_col {
            None
        } else {
            let next_grapheme = self.line.grapheme_at(self.start_col - 1);
            match &next_grapheme {
                Some((_, span)) => self.start_col = span.start,
                None => {}
            };
            next_grapheme
        }
    }
}

pub struct DisplayBuffer {
    range: Range<usize>,
    lines: Vec<DisplayLine>,
}

impl DisplayBuffer {
    pub fn empty() -> Self {
        Self {
            range: 0..0,
            lines: vec![],
        }
    }

    pub fn from_range(config: &Config, rope: &Rope, range: Range<usize>) -> Self {
        let tab_width = config.tab_width;
        let lines = Self::create_lines(rope, range.clone(), tab_width);

        Self {
            range: range,
            lines,
        }
    }

    fn create_lines(rope: &Rope, range: Range<usize>, tab_width: usize) -> Vec<DisplayLine> {
        range
            .map(|line_idx| DisplayLine::new(rope.line(line_idx), tab_width))
            .collect()
    }

    pub fn ensure_range(&mut self, config: &Config, rope: &Rope, range: Range<usize>) {
        if range.is_empty() {
            return;
        }

        if range.start >= self.range.start && range.end <= self.range.end {
            return;
        }

        let tab_width = config.tab_width;

        // Scroll up: new end overlaps with old view, but new start is higher
        if range.end > self.range.start
            && range.end <= self.range.end
            && range.start < self.range.start
        {
            let lines_to_add = range.start..self.range.start;
            let mut new_lines = Self::create_lines(rope, lines_to_add, tab_width);

            if self.should_truncate(self.range.end - range.end) {
                self.lines.truncate(range.end - self.range.start);
                self.range.end = range.end;
            };

            let mut old_lines = std::mem::take(&mut self.lines);
            new_lines.append(&mut old_lines);
            self.lines = new_lines;
            self.range.start = range.start;
        }
        // Scroll down: new start overlaps with old view, but new end is lower
        else if range.start >= self.range.start
            && range.start < self.range.end
            && range.end > self.range.end
        {
            let lines_to_add = self.range.end..range.end;
            let mut new_lines = Self::create_lines(rope, lines_to_add, tab_width);

            if self.should_truncate(range.start - self.range.start) {
                self.lines.drain(0..range.start - self.range.start);
                self.range.start = range.start;
            }

            self.lines.append(&mut new_lines);
            self.range.end = range.end;
        }
        // No overlap, build lines from scratch
        else {
            self.lines = Self::create_lines(rope, range.clone(), tab_width);
            self.range = range;
        }
    }

    fn should_truncate(&self, diff: usize) -> bool {
        diff as f64 > self.range.len() as f64 * 0.2
    }

    pub fn patch_range(&mut self, config: &Config, rope: &Rope, line_range: Range<usize>) {
        if line_range.start >= self.range.end || line_range.end <= self.range.start {
            return;
        }

        let start = line_range.start.max(self.range.start);
        let end = line_range.end.min(self.range.end);
        let patch_range = start..end;

        let tab_width = config.tab_width;
        let new_lines = Self::create_lines(rope, patch_range.clone(), tab_width);

        let vec_start = patch_range.start - self.range.start;
        let vec_end = patch_range.end - self.range.start;
        self.lines.splice(vec_start..vec_end, new_lines);
    }

    pub fn destroy_from(&mut self, line: usize) {
        if line >= self.range.end {
            return;
        }

        if line <= self.range.start {
            self.lines.clear();
            self.range = line..line;
            return;
        }

        self.range = self.range.start..line;
        self.lines.truncate(line - self.range.start);
    }

    pub fn ensure_line(&self, config: &Config, rope: &Rope, line_idx: usize) -> DisplayLineRef<'_> {
        if self.range.contains(&line_idx) {
            DisplayLineRef::Borrowed(&self.lines[line_idx - self.range.start])
        } else {
            let tab_width = config.tab_width;
            let line = Self::create_lines(rope, line_idx..line_idx + 1, tab_width)
                .into_iter()
                .next()
                .unwrap();
            DisplayLineRef::Owned(line)
        }
    }
}

pub enum DisplayLineRef<'a> {
    Borrowed(&'a DisplayLine),
    Owned(DisplayLine),
}

impl<'a> Deref for DisplayLineRef<'a> {
    type Target = DisplayLine;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(line) => line,
            Self::Owned(line) => line,
        }
    }
}
