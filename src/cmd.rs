mod motion;
mod operator;
mod secondary;
mod text_object;

pub use motion::Motion;
pub use operator::{EditOp, ExMode, InsertPoint, Operator, SearchOp, SysOp};
pub use secondary::Secondary;
pub use text_object::{Kind, Scope, TextObject};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    None,
    Motion {
        reps: Option<usize>,
        motion: Motion,
    },
    TextObject {
        reps: Option<usize>,
        text_object: TextObject,
    },
    Secondary(Secondary),
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

    pub fn reps(mut self, reps: usize) -> Self {
        self.reps = Some(reps);
        self
    }

    pub fn opt_reps(mut self, reps: Option<usize>) -> Self {
        self.reps = reps;
        self
    }

    pub fn reg(mut self, reg: char) -> Self {
        self.reg = Some(reg);
        self
    }

    pub fn opt_reg(mut self, reg: Option<char>) -> Self {
        self.reg = reg;
        self
    }

    pub fn motion(mut self, motion: Motion) -> Self {
        self.arg = Arg::Motion { reps: None, motion };
        self
    }

    pub fn rep_motion(mut self, reps: usize, motion: Motion) -> Self {
        self.arg = Arg::Motion {
            reps: Some(reps),
            motion,
        };
        self
    }

    pub fn text_object(mut self, reps: Option<usize>, text_object: TextObject) -> Self {
        self.arg = Arg::TextObject {
            reps: reps,
            text_object,
        };
        self
    }

    pub fn secondary(mut self, special: Secondary) -> Self {
        self.arg = Arg::Secondary(special);
        self
    }
}
