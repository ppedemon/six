pub enum Direction {
    Forward,
    Backward,
}

pub struct SearchDef {
  pat: String,
  direction: Direction,
  offset: usize,
}
