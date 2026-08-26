use crate::cmd::Cmd;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeatBuffer {
    last_cmd: Option<Cmd>,
}

impl RepeatBuffer {
    pub fn new() -> Self {
        Self { last_cmd: None }
    }

    pub fn last_cmd(&self) -> Option<Cmd> {
        self.last_cmd
    }

    pub fn save_last_cmd(&mut self, cmd: Cmd) {
        self.last_cmd = Some(cmd)
    }
}
