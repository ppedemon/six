use ropey::{Rope, RopeSlice};
use std::collections::{HashMap, hash_map::Entry};

use crate::cmd::EditOp;

#[derive(Debug, Clone)]
pub enum RegisterData {
    Char { rope: Rope },
    Line { rope: Rope },
    Block { rope: Rope },
}

impl RegisterData {
    pub fn char(rope: Rope) -> Self {
        Self::Char { rope }
    }

    pub fn line(rope: Rope) -> Self {
        Self::Line { rope }
    }

    pub fn block(ropes: Vec<Rope>) -> Self {
        let mut rope = Rope::new();
        soft_vstack(&mut rope, ropes);
        Self::Block { rope }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            RegisterData::Char { rope }
            | RegisterData::Line { rope }
            | RegisterData::Block { rope } => rope.len_chars() == 0,
        }
    }

    pub fn append(&mut self, incoming: RegisterData) {
        let current = std::mem::replace(self, Self::Char { rope: Rope::new() });

        *self = match (current, incoming) {
            (Self::Char { mut rope }, Self::Char { rope: other }) => {
                rope.append(other);
                Self::Char { rope }
            }
            (Self::Char { mut rope }, Self::Line { rope: other })
            | (Self::Line { mut rope }, Self::Char { rope: other })
            | (Self::Line { mut rope }, Self::Line { rope: other }) => {
                vstack(&mut rope, [other]);
                Self::Line { rope }
            }
            (Self::Block { mut rope }, Self::Block { rope: other }) => {
                vstack(&mut rope, [other]);
                Self::Line { rope }
            }
            (Self::Block { mut rope }, Self::Line { rope: other })
            | (Self::Block { mut rope }, Self::Char { rope: other }) => {
                vstack(&mut rope, [other]);
                Self::Line { rope }
            }
            (Self::Line { mut rope }, Self::Block { rope: other })
            | (Self::Char { mut rope }, Self::Block { rope: other }) => {
                vstack(&mut rope, [other]);
                Self::Line { rope }
            }
        };
    }
}

fn vstack(rope: &mut Rope, ropes: impl IntoIterator<Item = Rope>) {
    soft_vstack(rope, ropes);
    ensure_nl(rope);
}

fn soft_vstack(rope: &mut Rope, ropes: impl IntoIterator<Item = Rope>) {
    for r in ropes {
        ensure_nl(rope);
        rope.append(r);
    }
}

fn ensure_nl(rope: &mut Rope) {
    let len = rope.len_chars();
    if len > 0 && rope.char(len - 1) != '\n' {
        rope.insert_char(len, '\n');
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    Unnamed,
    Named(char),
    Append(char),
    Numbered(u8),
}

impl Register {
    pub const SMALL_DELETE: Self = Self::Named('-');
    pub const BLACKHOLE: Self = Self::Named('_');
    pub const LAST_INSERT: Self = Self::Named('.');

    pub fn from(c: char) -> Self {
        if c.is_ascii_digit() {
            Self::Numbered((c as u8) - b'0')
        } else if c.is_ascii_uppercase() {
            Self::Append(c.to_ascii_lowercase())
        } else {
            Self::Named(c)
        }
    }

    pub fn is_blackhole(&self) -> bool {
        self == &Self::BLACKHOLE
    }

    pub fn is_last_insert(&self) -> bool {
        self == &Self::LAST_INSERT
    }

    pub fn is_readonly(&self) -> bool {
        self.is_last_insert()
    }
}

#[derive(Debug)]
pub struct Registers {
    unnamed: Option<RegisterData>,
    named: HashMap<char, RegisterData>,
    numbered: [Option<RegisterData>; Self::NUM_REGS],
    last_insert: Vec<EditOp>,
}

impl Registers {
    const NUM_REGS: usize = 10;

    pub fn empty() -> Self {
        Self {
            unnamed: None,
            named: HashMap::new(),
            numbered: Default::default(),
            last_insert: Vec::new(),
        }
    }

    pub fn commit_insert_log(&mut self, ops: Vec<EditOp>) {
        self.last_insert = ops;
    }

    pub fn record_delete(&mut self, data: RegisterData) {
        for i in (2..Self::NUM_REGS).rev() {
            let data = std::mem::take(&mut self.numbered[i as usize - 1]);
            self.numbered[i as usize] = data;
        }
        self.numbered[1] = Some(data);
    }

    pub fn write(&mut self, reg: Register, data: RegisterData) {
        match reg {
            Register::Unnamed => self.unnamed = Some(data),
            Register::Named(c) => {
                self.named.insert(c, data);
            }
            Register::Append(c) => {
                let key = c.to_ascii_lowercase();
                match self.named.entry(key) {
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().append(data);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(data);
                    }
                }
            }
            Register::Numbered(i) => {
                if (i as usize) < Self::NUM_REGS {
                    self.numbered[i as usize] = Some(data);
                }
            }
        }
    }

    pub fn read(&self, reg: Register) -> Option<&RegisterData> {
        match reg {
            Register::Unnamed => self.unnamed.as_ref(),
            Register::Named(c) | Register::Append(c) => self.named.get(&c),
            Register::Numbered(i) => self.numbered.get(i as usize).and_then(Option::as_ref),
        }
    }

    pub fn last_insert(&self) -> &[EditOp] {
        &self.last_insert
    }

    pub fn record_small_delete(&mut self, reg: Option<char>, deleted: RopeSlice) {
        let r = reg.map_or(Register::SMALL_DELETE, Register::from);

        if r.is_blackhole() || r.is_readonly() {
            return;
        }

        let data = RegisterData::char(deleted.into());
        self.write(Register::Unnamed, data.clone());
        self.write(r, data);
    }

    pub fn record_yank(&mut self, reg: Option<char>, data: RegisterData) {
        let r = reg.map_or(Register::Numbered(0), Register::from);

        if r.is_blackhole() || r.is_readonly() {
            return;
        }

        self.write(Register::Unnamed, data.clone());
        self.write(r, data);
    }
}
