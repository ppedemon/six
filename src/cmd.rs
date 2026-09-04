mod insert;
mod motion;
mod operator;
mod text_object;

pub use insert::{EditOp, InsertOp};
pub use motion::Motion;
pub use operator::{ExMode, ImmediateOp, InsertPoint, InteractiveOp, Operator, SysOp};
pub use text_object::{TextObject, TextObjectKind, TextObjectScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionMode {
    Charwise,
    Linewise,
    Blockwise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    None,
    Motion {
        reps: Option<usize>,
        mode: Option<MotionMode>,
        motion: Motion,
    },
    TextObject {
        reps: Option<usize>,
        mode: Option<MotionMode>,
        text_object: TextObject,
    },
}

impl Arg {
    pub fn motion(reps: Option<usize>, mode: Option<MotionMode>, motion: Motion) -> Self {
        Self::Motion { reps, mode, motion }
    }

    pub fn text_object(
        reps: Option<usize>,
        mode: Option<MotionMode>,
        text_object: TextObject,
    ) -> Self {
        Self::TextObject {
            reps,
            mode,
            text_object,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cmd {
    pub reps: Option<usize>, // Set if reps specified, otherwise a None value to be interpreted by the operator
    pub reg: Option<char>,   // Ignored if op doesn't use a register, None = default register
    pub op: Operator,
    pub arg: Arg,
}

impl Cmd {
    pub fn new(op: Operator) -> Self {
        Self {
            reps: None,
            reg: None,
            op,
            arg: Arg::None,
        }
    }

    pub fn reps(mut self, reps: Option<usize>) -> Self {
        self.reps = reps;
        self
    }

    pub fn reg(mut self, reg: Option<char>) -> Self {
        self.reg = reg;
        self
    }

    pub fn arg(mut self, arg: Arg) -> Self {
        self.arg = arg;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TxnItem {
    Cmd(Cmd),
    EditOp(EditOp),
}

pub type Txn<'a> = &'a [TxnItem];

impl From<Cmd> for TxnItem {
    fn from(cmd: Cmd) -> Self {
        Self::Cmd(cmd)
    }
}

impl From<EditOp> for TxnItem {
    fn from(edit_op: EditOp) -> Self {
        Self::EditOp(edit_op)
    }
}
