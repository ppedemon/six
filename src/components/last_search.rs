use crate::cmd::{Cmd, Motion, Operator};

enum Direction {
    Backward,
    Foward,
}

pub struct LastSearch {
    last_char_search: Option<Motion>,
    last_pat: Option<String>,
    direction: Option<Direction>,
    offset: usize,
    cmd: Cmd,
}

impl LastSearch {
    pub fn empty() -> Self {
        Self {
            last_char_search: None,
            last_pat: None,
            direction: None,
            offset: 0,
            cmd: Cmd::new(Operator::Nop),
        }
    }

    pub fn save_char_search(&mut self, m: Motion) {
        assert!(matches!(
            m,
            Motion::FindNextChar(_)
                | Motion::FindPrevChar(_)
                | Motion::TillNextChar(_)
                | Motion::TillPrevChar(_)
        ));
        self.last_char_search = Some(m);
    }

    pub fn last_char_search(&self) -> Option<Motion> {
        self.last_char_search
    }

    pub fn init_fwd_pat_search(&mut self, cmd: Cmd) {
        self.direction = Some(Direction::Foward);
        self.cmd = cmd;
    }

    pub fn init_bwd_pat_search(&mut self, cmd: Cmd) {
        self.direction = Some(Direction::Backward);
        self.cmd = cmd;
    }
}
