use crate::cmd::Motion;

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

// System commands:
// Mode switching, general buffer ops (like ZZ and ZQ),
// TODO add window management here
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysOp {
    EnterNormal,
    EnterEx(ExMode),
    EnterInsert(InsertPoint),
    HardQuit,
    CondWriteAndQuit,
}

// Interactive commands:
// Do something, then enter insert mode. For example, o,O,c
// TODO add c,C,s
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractiveOp {
    OpenAbove,
    OpenBelow,
}

// Immediate commands: just mutate the text or registers.
// Example: x, d, y, p
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImmediateOp {
    Delete,
    Backspace,
    Join,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Nop,
    Move(Motion),
    Sys(SysOp),
    Interactive(InteractiveOp),
    Immediate(ImmediateOp),
}

impl From<Motion> for Operator {
    fn from(motion: Motion) -> Self {
        Self::Move(motion)
    }
}

impl From<SysOp> for Operator {
    fn from(sys_op: SysOp) -> Self {
        Self::Sys(sys_op)
    }
}

impl From<InteractiveOp> for Operator {
    fn from(interactive_op: InteractiveOp) -> Self {
        Self::Interactive(interactive_op)
    }
}

impl From<ImmediateOp> for Operator {
    fn from(immediate_op: ImmediateOp) -> Self {
        Self::Immediate(immediate_op)
    }
}

impl Operator {
    // Return Some(c) iif the operator supports "doubling" to act on lines
    pub fn line_arg_char(&self) -> Option<char> {
        // TODO c, d, and y should return Some('c'), Some('d') and Some('y'), respectively.
        None
    }
}
