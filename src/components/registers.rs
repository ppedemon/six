use ropey::{Rope, RopeSlice};
use std::collections::HashMap;

use crate::cmd::EditOp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YankFlavor {
    Character,
    Line,
    Block { width: usize },
}

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
}

pub enum Register {
    Unnamed,
    Named(char),
    Numbered(u8),
}

pub struct Registers {
    unnamed: Option<RegisterData>,
    named: HashMap<char, RegisterData>,
    numbered: [Option<RegisterData>; Self::NUM_REGS],
    small_delete: Option<RegisterData>,
    last_insert: Vec<EditOp>,
}

impl Registers {
    const NUM_REGS: usize = 10;

    pub fn empty() -> Self {
        Self {
            unnamed: None,
            named: HashMap::new(),
            numbered: Default::default(),
            small_delete: None,
            last_insert: Vec::new(),
        }
    }

    pub fn commit_insert_log(&mut self, ops: Vec<EditOp>) {
        self.last_insert = ops;
    }

    pub fn record_small_delete(&mut self, rope_slice: RopeSlice, flavor: YankFlavor) {
        let data = RegisterData::new(rope_slice, flavor);
        self.small_delete = Some(data);
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
            Register::Numbered(i) => {
                if (i as usize) < Self::NUM_REGS{
                    self.numbered[i as usize] = Some(data);
                }
            }
        }
    }

    pub fn read(&self, reg: Register) -> Option<&RegisterData> {
        match reg {
            Register::Unnamed => self.unnamed.as_ref(),
            Register::Named(c) => self.named.get(&c),
            Register::Numbered(i) => self.numbered.get(i as usize).and_then(Option::as_ref),
        }
    }

    pub fn last_insert(&self) -> &[EditOp] {
        &self.last_insert
    }

    pub fn small_delete(&self) -> Option<&RegisterData> {
        self.small_delete.as_ref()
    }
}
