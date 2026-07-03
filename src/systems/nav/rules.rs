use crate::components::DisplayLine;

pub trait NavRules {
    fn max_allowed_width(line: &DisplayLine) -> usize;
    fn first_non_blank(line: &DisplayLine) -> usize;

    fn prev_col(line: &DisplayLine, col: usize) -> usize;
    fn next_col(line: &DisplayLine, col: usize) -> usize;
    fn snap_col(line: &DisplayLine, col: usize) -> usize;
}

pub struct NormalNav;

impl NavRules for NormalNav {
    fn max_allowed_width(line: &DisplayLine) -> usize {
        line.display_width.saturating_sub(1)
    }

    fn first_non_blank(line: &DisplayLine) -> usize {
        line.first_non_blank()
    }

    fn prev_col(line: &DisplayLine, col: usize) -> usize {
        line.prev_col(col)
    }

    fn next_col(line: &DisplayLine, col: usize) -> usize {
        line.next_col(col)
    }

    fn snap_col(line: &DisplayLine, col: usize) -> usize {
        line.snap_col(col)
    }
}

pub struct InsertNav;

impl NavRules for InsertNav {
    fn max_allowed_width(line: &DisplayLine) -> usize {
        line.display_width
    }

    fn first_non_blank(line: &DisplayLine) -> usize {
        line.first_insert_non_blank()
    }

    fn prev_col(line: &DisplayLine, col: usize) -> usize {
        line.prev_insert_col(col)
    }

    fn next_col(line: &DisplayLine, col: usize) -> usize {
        line.next_insert_col(col)
    }

    fn snap_col(line: &DisplayLine, col: usize) -> usize {
        line.snap_insert_col(col)
    }
}
