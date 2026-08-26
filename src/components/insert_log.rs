use crate::cmd::EditOp;

pub struct InsertLog {
    poisoned: bool,
    repetable: bool,
    log: Vec<EditOp>,
}

impl InsertLog {
    const DEF_CAPACITY: usize = 1024;

    pub fn new() -> Self {
        Self {
            poisoned: false,
            repetable: true,
            log: Vec::with_capacity(Self::DEF_CAPACITY),
        }
    }

    pub fn init(&mut self) {
        self.poisoned = false;
        self.repetable = true;
        self.log.clear();
    }

    pub fn poison(&mut self) {
        self.poisoned = true;
        self.repetable = false;
    }

    pub fn append(&mut self, op: EditOp) {
        if self.poisoned {
            self.poisoned = false;
            self.log.clear();
        }
        self.log.push(op);
    }

    pub fn take_log(&mut self) -> (Vec<EditOp>, bool) {
        let edit_ops = std::mem::take(&mut self.log);
        (edit_ops, self.repetable)
    }
}
