use std::ops::Range;

use ropey::Rope;

use crate::components::{
    Change, Marks, UndoSummary,
    undo::{Action, LastEditedLine, UndoStack, UndoTxn},
};

// Mutable view of a buffer
pub trait MutBuffer {
    fn rope(&self) -> &Rope;
    fn insert_char(&mut self, char_idx: usize, ch: char);
    fn insert(&mut self, char_idx: usize, text: &str);
    fn insert_rope(&mut self, char_idx: usize, rope: &Rope);
    fn insert_iter(&mut self, char_idx: usize, iter: impl IntoIterator<Item = char>);
    fn remove(&mut self, char_range: Range<usize>);
    fn undo(&mut self) -> Option<UndoSummary>;
    fn undo_line(&mut self) -> Option<usize>;
}

pub struct Buffer {
    marks: Marks,
    undo_stack: UndoStack,
    last_edited_line: LastEditedLine,
    dirty: bool,
    rope: Rope,
}

impl Buffer {
    pub fn new(rope: Rope) -> Self {
        Self {
            marks: Marks::new(),
            undo_stack: UndoStack::new(),
            last_edited_line: LastEditedLine::new(),
            dirty: false,
            rope,
        }
    }

    pub fn empty() -> Self {
        Self {
            marks: Marks::new(),
            undo_stack: UndoStack::new(),
            last_edited_line: LastEditedLine::new(),
            dirty: false,
            rope: Rope::new(),
        }
    }

    pub fn marks(&self) -> &Marks {
        &self.marks
    }

    pub fn marks_mut(&mut self) -> &mut Marks {
        &mut self.marks
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn saved(&mut self) {
        self.dirty = false;
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn edit(&mut self) -> impl MutBuffer {
        SessionMutableBuffer { buffer: self }
    }

    pub fn commit_undo_txn(&mut self) {
        self.undo_stack.commit();

        // vi seems to have a quirk here, which this code tries to mimic...
        // When you do a multiline edit, only take into account the last
        // line if it wasn't previously empty. I don't think this is the
        // exact rule, but f*ck it. I declare this good enough.
        let multi_line = self
            .undo_stack
            .peek()
            .is_some_and(|txn| multi_line(txn, &self.rope));
        if multi_line && self.last_edited_line.line.is_empty() {
            self.last_edited_line.clear();
        }
    }
}

// Define how to mutate a Buffer
struct SessionMutableBuffer<'a> {
    buffer: &'a mut Buffer,
}

impl<'a> MutBuffer for SessionMutableBuffer<'a> {
    fn rope(&self) -> &Rope {
        &self.buffer.rope
    }

    fn insert_char(&mut self, char_idx: usize, ch: char) {
        let buffer = &mut self.buffer;

        buffer.last_edited_line.will_edit(&buffer.rope, char_idx);
        buffer.rope.insert_char(char_idx, ch);
        buffer.undo_stack.insert_char(buffer.dirty, char_idx, ch);

        buffer.marks.adjust(Change::insert(char_idx, 1));
        buffer.dirty = true;
    }

    fn insert(&mut self, char_idx: usize, text: &str) {
        if text.is_empty() {
            return;
        }

        let buffer = &mut self.buffer;

        buffer.last_edited_line.will_edit(&buffer.rope, char_idx);
        buffer.rope.insert(char_idx, text);
        buffer.undo_stack.insert_str(buffer.dirty, char_idx, text);

        buffer
            .marks
            .adjust(Change::insert(char_idx, text.chars().count()));
        buffer.dirty = true;
    }

    fn insert_iter(&mut self, char_idx: usize, iter: impl IntoIterator<Item = char>) {
        let buffer = &mut self.buffer;

        let mut len = 0;
        let mut first = true;
        for (i, c) in iter.into_iter().enumerate() {
            if first {
                first = false;
                buffer
                    .last_edited_line
                    .will_edit(&buffer.rope, char_idx + i);
            }
            buffer.rope.insert_char(char_idx + i, c);
            buffer.undo_stack.insert_char(buffer.dirty, char_idx + i, c);
            len += 1;
        }

        buffer.marks.adjust(Change::insert(char_idx, len));
        buffer.dirty = true;
    }

    fn insert_rope(&mut self, char_idx: usize, rope: &Rope) {
        if rope.len_chars() == 0 {
            return;
        }

        let buffer = &mut self.buffer;
        buffer.last_edited_line.will_edit(&buffer.rope, char_idx);

        let tail = buffer.rope.split_off(char_idx);
        buffer.rope.append(rope.clone());
        buffer.rope.append(tail);
        buffer.undo_stack.insert_rope(buffer.dirty, char_idx, rope);

        buffer
            .marks
            .adjust(Change::insert(char_idx, rope.len_chars()));
        buffer.dirty = true;
    }

    fn remove(&mut self, char_range: Range<usize>) {
        let buffer = &mut self.buffer;
        buffer
            .last_edited_line
            .will_edit(&buffer.rope, char_range.start);

        if char_range.len() == 1 {
            let c = buffer.rope.char(char_range.start);
            buffer
                .undo_stack
                .remove_char(buffer.dirty, char_range.clone(), c);
        } else {
            let text = buffer.rope.slice(char_range.clone()).to_string();
            buffer
                .undo_stack
                .remove(buffer.dirty, char_range.clone(), &text);
        }

        buffer.rope.remove(char_range.clone());
        buffer.marks.adjust(Change::delete(char_range));
        buffer.dirty = true;
    }

    fn undo(&mut self) -> Option<UndoSummary> {
        let buffer = &mut self.buffer;

        if let Some(txn) = buffer.undo_stack.pop() {
            buffer.dirty = txn.dirty;

            let mut summary = UndoSummary::default();
            for chunk in txn.data.into_iter().rev() {
                let s = mk_summary(
                    &buffer.rope,
                    txn.timestamp_ms,
                    txn.change_id,
                    chunk.char_idx,
                    chunk.num_lines,
                );
                summary.merge(s);

                match chunk.action {
                    Action::Insert => {
                        let end_idx = chunk.char_idx + chunk.len_chars;
                        buffer.rope.remove(chunk.char_idx..end_idx);
                        buffer.marks.adjust(Change::delete(chunk.char_idx..end_idx));
                    }
                    Action::Delete => {
                        buffer.rope.insert(chunk.char_idx, &chunk.text);
                        buffer
                            .marks
                            .adjust(Change::insert(chunk.char_idx, chunk.len_chars));
                    }
                }
            }
            return Some(summary);
        }

        None
    }

    fn undo_line(&mut self) -> Option<usize> {
        if self.buffer.last_edited_line.is_empty() {
            None
        } else {
            let line = self.buffer.last_edited_line.take();
            let line_idx = self.buffer.last_edited_line.line_idx;

            let start_idx = self.buffer.rope.line_to_char(line_idx);
            let end_idx = self
                .buffer
                .rope
                .line_to_char(line_idx.saturating_add(1).min(self.buffer.rope.len_lines()));

            self.buffer.edit().remove(start_idx..end_idx);
            self.buffer.edit().insert(start_idx, &line);
            self.buffer.commit_undo_txn();

            Some(line_idx)
        }
    }
}

fn mk_summary(
    rope: &Rope,
    timestamp_ms: u64,
    change_id: u64,
    char_idx: usize,
    num_lines: usize,
) -> UndoSummary {
    let line_idx = rope.char_to_line(char_idx);
    if num_lines > 1 {
        UndoSummary::destroy_from(timestamp_ms, change_id, char_idx, line_idx)
    } else {
        UndoSummary::single_line(timestamp_ms, change_id, char_idx, line_idx)
    }
}

fn multi_line(txn: &UndoTxn, rope: &Rope) -> bool {
    let mut line = None;
    for chunk in &txn.data {
        if chunk.num_lines != 1 {
            return true;
        }

        let line_idx = rope.char_to_line(chunk.char_idx);
        match line {
            None => line = Some(line_idx),
            Some(line) if line != line_idx => return true,
            _ => {}
        }
    }
    false
}
