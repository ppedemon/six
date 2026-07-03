use crate::normal::EditOp;

pub struct InsertLog {
    pub reps: usize,
    pub log: Vec<EditOp>,
}

impl InsertLog {
    const DEF_CAPACITY: usize = 1024;

    pub fn new() -> Self {
        Self {
            reps: 0,
            log: Vec::with_capacity(Self::DEF_CAPACITY),
        }
    }

    pub fn reset(&mut self) {
        self.reps = 0;
        self.log.clear();
    }

    pub fn init(&mut self, reps: usize) {
        self.reset();
        self.reps = reps;
    }

    pub fn append(&mut self, op: EditOp) {
        if self.reps > 0 {
            self.log.push(op);
        }
    }

    pub fn log(&self) -> &Vec<EditOp> {
        &self.log
    }
}
