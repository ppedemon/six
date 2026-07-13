use crate::cmd::{Cmd, EditOp, Operator, SysOp::EnterInsert};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdItem {
    None,
    Plain(Cmd),
    PartialInteraction(Cmd),
    FullInteraction(Cmd, Vec<EditOp>),
}

pub struct RepeatBuffer {
    pub cmd_item: CmdItem,
}

impl RepeatBuffer {
    pub fn new() -> Self {
        Self {
            cmd_item: CmdItem::None,
        }
    }

    pub fn start_interaction(&mut self, cmd: Cmd) {
        match self.cmd_item {
            CmdItem::PartialInteraction(partial_cmd) => {
                if !matches!(cmd.op, Operator::Sys(EnterInsert(_))) {
                    panic!("Invalid interaction nesting: {partial_cmd:?} -> {cmd:?}");
                }
            }
            _ => self.cmd_item = CmdItem::PartialInteraction(cmd),
        }
    }

    pub fn finish_interaction(&mut self, ops: Vec<EditOp>) {
        match self.cmd_item {
            CmdItem::PartialInteraction(cmd) => self.cmd_item = CmdItem::FullInteraction(cmd, ops),
            _ => panic!(
                "No interaction to finish, current cmd is {:?}",
                self.cmd_item
            ),
        }
    }
}
