use crate::cmd::{Cmd, EditOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatBufferItem {
    None,
    Immediate(Cmd),
    Partial(Cmd),
    Interactive(Cmd, Vec<EditOp>),
}

pub struct RepeatBuffer {
    pub item: RepeatBufferItem,
}

impl RepeatBuffer {
    pub fn new() -> Self {
        Self {
            item: RepeatBufferItem::None,
        }
    }

    pub fn start_interaction(&mut self, cmd: Cmd) {
        assert!(
            !matches!(self.item, RepeatBufferItem::Partial(_)),
            "Invalid interaction nesting: {:?} -> {cmd:?}",
            self.item
        );
        self.item = RepeatBufferItem::Partial(cmd)
    }

    pub fn finish_interaction(&mut self, ops: Vec<EditOp>) {
        match self.item {
            RepeatBufferItem::Partial(cmd) => self.item = RepeatBufferItem::Interactive(cmd, ops),
            _ => panic!("No interaction to finish for cmd {:?}", self.item),
        }
    }
}
