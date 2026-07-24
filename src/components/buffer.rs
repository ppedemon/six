use std::ops::Range;

use ropey::Rope;

use crate::components::{Change, Marks};

// Mutable view of a buffer
pub trait MutBuffer {
    fn rope(&self) -> &Rope;
    fn insert_char(&mut self, char_idx: usize, ch: char);
    fn insert(&mut self, char_idx: usize, text: &str);
    fn remove(&mut self, char_range: Range<usize>);
}

pub struct Buffer {
    marks: Marks,
    dirty: bool,
    rope: Rope,
}

impl Buffer {
    pub fn new(rope: Rope) -> Self {
        Self {
            marks: Marks::new(),
            dirty: false,
            rope,
        }
    }

    pub fn empty() -> Self {
        Self {
            marks: Marks::new(),
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
        self.buffer.marks.adjust(Change::insert(char_idx, 1));
        self.buffer.dirty = true;
    }

    fn insert(&mut self, char_idx: usize, text: &str) {
        self.buffer.rope.insert(char_idx, text);
        self.buffer
            .marks
            .adjust(Change::insert(char_idx, text.len()));
        self.buffer.dirty = true;
    }

    fn remove(&mut self, char_range: Range<usize>) {
        self.buffer.rope.remove(char_range.clone());
        self.buffer.marks.adjust(Change::delete(char_range));
        self.buffer.dirty = true;
    }
}
