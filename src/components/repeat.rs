use crate::{cmd::Cmd, components::Coords};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YankShape {
    Char {
        num_lines: usize,
        end_col: usize,
        inclusive: bool,
    },
    Line {
        num_lines: usize,
    },
    Block {
        rows: usize,
        cols: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YankData {
    pub start: Coords,
    pub shape: YankShape,
}

impl YankData {
    pub fn new(start: Coords, shape: YankShape) -> Self {
        Self { start, shape }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeatBuffer {
    last_cmd: Option<Cmd>,
    last_yank: Option<YankData>,
}

impl RepeatBuffer {
    pub fn new() -> Self {
        Self {
            last_cmd: None,
            last_yank: None,
        }
    }

    pub fn last_cmd(&self) -> Option<Cmd> {
        self.last_cmd
    }

    pub fn save_last_cmd(&mut self, cmd: Cmd) {
        self.last_cmd = Some(cmd)
    }

    pub fn last_yank(&self) -> Option<YankData> {
        self.last_yank
    }

    pub fn save_last_yank(&mut self, yank_data: YankData) {
        self.last_yank = Some(yank_data)
    }
}
