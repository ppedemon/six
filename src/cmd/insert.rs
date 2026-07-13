use crate::cmd::Motion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOp {
    Esc,
    InsertChar(char),
    Tab,
    Enter,
    Backspace,
    Delete,
}

pub enum InsertOp {
    Edit(EditOp),
    Move(Motion),
}

impl From<EditOp> for InsertOp {
    fn from(edit_op: EditOp) -> Self {
        InsertOp::Edit(edit_op)
    }
}

impl From<Motion> for InsertOp {
    fn from(motion: Motion) -> Self {
        InsertOp::Move(motion)
    }
}
