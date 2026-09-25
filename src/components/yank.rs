use crate::{cmd::MotionMode, components::Coords};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YankData {
    pub start: Coords,
    pub end: Coords,
    pub mode: MotionMode,
    pub inclusive: bool,
}

impl YankData {
    pub fn new(start: Coords, end: Coords, mode: MotionMode, inclusive: bool) -> Self {
        let mut start = start;
        let mut end = end;
        if start > end {
            std::mem::swap(&mut start, &mut end);
        }
        Self {
            start,
            end,
            mode,
            inclusive,
        }
    }

    pub fn num_lines(&self) -> usize {
        self.end.row - self.start.row + 1
    }

    pub fn end_col(&self) -> usize {
        self.end.col
    }

    pub fn block(&self) -> (usize, usize) {
        let rows = self.num_lines();
        let cols = self.start.col.max(self.end.col) - self.start.col.min(self.end.col);
        (rows, cols)
    }
}
