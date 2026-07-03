use crate::normal::{Motion, Operator, Secondary, TextObject};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    None,
    Motion(Motion),
    TextObject(TextObject),
    Secondary(Secondary),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalCmd {
    pub reps: usize,
    pub op: Operator,
    pub target: Target,
}

impl NormalCmd {
    pub fn new(op: Operator) -> Self {
        Self {
            reps: 1,
            op,
            target: Target::None,
        }
    }

    pub fn reps(mut self, reps: usize) -> Self {
        self.reps = reps;
        self
    }

    pub fn motion(mut self, motion: Motion) -> Self {
        self.target = Target::Motion(motion);
        self
    }

    pub fn text_object(mut self, text_object: TextObject) -> Self {
        self.target = Target::TextObject(text_object);
        self
    }

    pub fn special(mut self, special: Secondary) -> Self {
        self.target = Target::Secondary(special);
        self
    }
}
