use crate::normal::Motion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOp {
    InsertChar(char),
    Tab,
    Enter,
    Backspace,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertPoint {
    Curr,
    Next,
    First,
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExMode {
    Colon,
    SearchForward,
    SearchBackward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysOp {
    EnterNormal,
    EnterEx(ExMode),
    EnterInsert(InsertPoint),
    BufferOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Nop,
    Sys(SysOp),
    Move(Motion),
    Edit(EditOp),
}

impl From<SysOp> for Operator {
    fn from(sys_op: SysOp) -> Self {
        Self::Sys(sys_op)
    }
}

impl From<Motion> for Operator {
    fn from(motion: Motion) -> Self {
        Self::Move(motion)
    }
}

impl From<EditOp> for Operator {
    fn from(edit_op: EditOp) -> Self {
        Self::Edit(edit_op)
    }
}

impl Operator {
    pub fn needs_target(&self) -> bool {
        match self {
            Self::Sys(SysOp::BufferOp) => true,
            _ => false,
        }
    }
}
