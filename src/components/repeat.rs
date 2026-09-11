use crate::cmd::Cmd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YankShape {
    Char { num_lines: usize, end_col: usize },
    Line { num_lines: usize },
    Block { rows: usize, cols: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeatBuffer {
    last_cmd: Option<Cmd>,
    last_yank_shape: Option<YankShape>,
}

impl RepeatBuffer {
    pub fn new() -> Self {
        Self {
            last_cmd: None,
            last_yank_shape: None,
        }
    }

    pub fn last_cmd(&self) -> Option<Cmd> {
        self.last_cmd
    }

    pub fn save_last_cmd(&mut self, cmd: Cmd) {
        self.last_cmd = Some(cmd)
    }

    pub fn last_yank_shape(&self) -> Option<YankShape> {
        self.last_yank_shape
    }

    pub fn save_last_yank_shape(&mut self, yank_shape: YankShape) {
        self.last_yank_shape = Some(yank_shape)
    }
}
