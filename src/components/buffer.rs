use ropey::Rope;

pub struct Buffer {
    pub dirty: bool,
    pub rope: Rope,
}

impl Buffer {
    pub fn new(rope: Rope) -> Self {
        Self { dirty: false, rope }
    }

    pub fn empty() -> Self {
        Self {
            dirty: false,
            rope: Rope::new(),
        }
    }
}
