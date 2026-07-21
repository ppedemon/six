use ropey::{Rope, RopeSlice};
use std::collections::HashMap;

use crate::cmd::EditOp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YankFlavor {
    Character,
    Line,
    Block { width: usize },
}

#[derive(Debug)]
pub struct RegisterData {
    pub rope: Rope,
    pub flavor: YankFlavor,
}

impl RegisterData {
    pub fn new(rope_slice: RopeSlice, flavor: YankFlavor) -> Self {
        Self {
            rope: rope_slice.into(),
            flavor,
        }
    }

    pub fn append(&mut self, rope_slice: RopeSlice, flavor: YankFlavor) {
        self.rope.append(rope_slice.into());
        self.flavor = match (flavor, self.flavor) {
            (YankFlavor::Character, YankFlavor::Character) => YankFlavor::Character,
            _ => YankFlavor::Line,
        };
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

    pub fn record_delete(&mut self, rope_slice: RopeSlice, flavor: YankFlavor) {
        for i in (2..Self::NUM_REGS).rev() {
            let data = std::mem::take(&mut self.numbered[i as usize - 1]);
            self.numbered[i as usize] = data;
        }
        let data = RegisterData::new(rope_slice, flavor);
        self.numbered[1] = Some(data);
    }

    pub fn write(&mut self, reg: Register, rope_slice: RopeSlice, flavor: YankFlavor) {
        let data = RegisterData::new(rope_slice, flavor);
        match reg {
            Register::Unnamed => self.unnamed = Some(data),
            Register::Named(c) => {
                self.named.insert(c, data);
            }
            Register::Append(c) => {
                let key = c.to_ascii_lowercase();
                self.named
                    .entry(c)
                    .and_modify(|v| v.append(rope_slice, flavor))
                    .or_insert(data);
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

    pub fn small_delete(&mut self, reg: Option<char>, deleted: RopeSlice) {
        let r = reg.map_or(Register::SMALL_DELETE, Register::from);

        if r.is_blackhole() || r.is_readonly() {
            return;
        }

        let flavor = YankFlavor::Character;
        self.write(Register::Unnamed, deleted, flavor);
        self.write(r, deleted, flavor);
    }
}
