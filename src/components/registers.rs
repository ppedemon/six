use ropey::Rope;
use std::collections::HashMap;

use crate::cmd::EditOp;

pub enum YankFlavor {
    Character,
    Line,
    Block { width: usize },
}

pub struct RegisterData {
    rope: Rope,
    flavor: YankFlavor,
}

pub struct Registers {
    pub data_registers: HashMap<char, RegisterData>,
    pub last_insert: Vec<EditOp>,
}

impl Registers {
    pub fn empty() -> Self {
        Self {
            data_registers: HashMap::new(),
            last_insert: Vec::new(),
        }
    }

    pub fn commit_insert_log(&mut self, ops: Vec<EditOp>) {
        self.last_insert = ops;
    }
}
