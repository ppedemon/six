use std::ops::Range;

use ropey::Rope;

use super::undo::UndoStack;
use crate::components::{
    Change, Marks,
    undo::{UndoChunk, UndoTxnMeta},
};

// Mutable view of a buffer
pub trait MutBuffer {
    fn rope(&self) -> &Rope;
    fn insert_char(&mut self, char_idx: usize, ch: char);
    fn insert(&mut self, char_idx: usize, text: &str);
    fn insert_rope(&mut self, char_idx: usize, rope: &Rope);
    fn insert_iter(&mut self, char_idx: usize, iter: impl IntoIterator<Item = char>);
    fn remove(&mut self, char_range: Range<usize>);
    fn undo(&mut self) -> Option<UndoTxnMeta>;
}

pub struct Buffer {
    marks: Marks,
    undo_stack: UndoStack,
    dirty: bool,
    rope: Rope,
}

impl Buffer {
    pub fn new(rope: Rope) -> Self {
        Self {
            marks: Marks::new(),
            undo_stack: UndoStack::new(),
            dirty: false,
            rope,
        }
    }

    pub fn empty() -> Self {
        Self {
            marks: Marks::new(),
            undo_stack: UndoStack::new(),
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
        self.buffer.rope.insert_char(char_idx, ch);
        self.buffer.undo_stack.insert_char(char_idx, ch);
        self.buffer.marks.adjust(Change::insert(char_idx, 1));
        self.buffer.dirty = true;
    }

    fn insert(&mut self, char_idx: usize, text: &str) {
        if text.is_empty() {
            return;
        }

        self.buffer.rope.insert(char_idx, text);
        self.buffer.undo_stack.insert_str(char_idx, text);

        self.buffer
            .marks
            .adjust(Change::insert(char_idx, text.chars().count()));
        self.buffer.dirty = true;
    }

    fn insert_iter(&mut self, char_idx: usize, iter: impl IntoIterator<Item = char>) {
        let mut len = 0;

        for (i, c) in iter.into_iter().enumerate() {
            self.buffer.rope.insert_char(char_idx + i, c);
            self.buffer.undo_stack.insert_char(char_idx + i, c);
            len += 1;
        }

        self.buffer.marks.adjust(Change::insert(char_idx, len));
    }

    fn insert_rope(&mut self, char_idx: usize, rope: &Rope) {
        if rope.len_chars() == 0 {
            return;
        }

        let tail = self.buffer.rope.split_off(char_idx);
        self.buffer.rope.append(rope.clone());
        self.buffer.rope.append(tail);
        self.buffer.undo_stack.insert_rope(char_idx, rope);

        self.buffer
            .marks
            .adjust(Change::insert(char_idx, rope.len_chars()));
        self.buffer.dirty = true;
    }

    fn remove(&mut self, char_range: Range<usize>) {
        if char_range.len() == 1 {
            let c = self.buffer.rope.char(char_range.start);
            self.buffer.undo_stack.remove_char(char_range.clone(), c);
        } else {
            let text = self.buffer.rope.slice(char_range.clone()).to_string();
            self.buffer.undo_stack.remove(char_range.clone(), &text);
        }

        self.buffer.rope.remove(char_range.clone());
        self.buffer.marks.adjust(Change::delete(char_range));
        self.buffer.dirty = true;
    }

    fn undo(&mut self) -> Option<UndoTxnMeta> {
        let buffer = &mut self.buffer;
        
        if let Some(txn) = buffer.undo_stack.pop() {
            let range = Some(txn.range());
            for chunk in txn.data.into_iter().rev() {
                match chunk {
                    UndoChunk::Insert {
                        char_idx,
                        text,
                        len_chars,
                        num_lines,
                    } => {
                        let end_idx = char_idx + len_chars;
                        buffer.rope.remove(char_idx..end_idx);
                        buffer.marks.adjust(Change::delete(char_idx..end_idx));
                    }
                    UndoChunk::Delete {
                        char_idx,
                        text,
                        len_chars,
                        num_lines,
                    } => {
                        buffer.rope.insert(char_idx, &text);
                        buffer.marks.adjust(Change::insert(char_idx, len_chars));
                    }
                }
            }
            return range;
        }

        None
    }
}
