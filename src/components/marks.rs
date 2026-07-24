use std::ops::Range;

pub enum Change {
    Insert { at: usize, len: usize },
    Delete { range: Range<usize> },
}

impl Change {
    pub fn insert(at: usize, len: usize) -> Self {
        Self::Insert { at, len }
    }

    pub fn delete(range: Range<usize>) -> Self {
        Self::Delete { range }
    }
}

pub struct Marks {
    named: [Option<usize>; 52],
}

impl Marks {
    pub fn new() -> Self {
        Self { named: [None; 52] }
    }

    pub fn write(&mut self, c: char, char_idx: usize) {
        match idx(c) {
            Some(idx) => self.named[idx] = Some(char_idx),
            None => {}
        }
    }

    pub fn read(&self, c: char) -> Option<usize> {
        self.named[idx(c)?]
    }

    pub fn delete(&mut self, marks: &[char]) {
        for c in marks {
            match idx(*c) {
                None => {}
                Some(idx) => self.named[idx] = None,
            }
        }
    }

    pub fn adjust(&mut self, change: Change) {
        for char_idx in self.named.iter_mut() {
            match char_idx {
                None => {}
                Some(idx) => match change {
                    Change::Insert { at, len } => {
                        if *idx >= at {
                            *idx += len
                        }
                    }
                    Change::Delete { ref range } => {
                        if range.contains(idx) {
                            *idx = range.start.saturating_sub(1);
                        } else if *idx >= range.end {
                            *idx -= range.len();
                        }
                    }
                },
            }
        }
    }
}

fn idx(c: char) -> Option<usize> {
    match c {
        'a'..='z' => Some((c as u8 - b'a') as usize),
        _ => None,
    }
}
