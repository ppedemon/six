use crate::{cmd::{Cmd, Motion, Operator}, ex::SearchDef};

enum Direction {
    Backward,
    Foward,
}

pub struct LastSearch {
    last_char_search: Option<Motion>,
    search_def: Option<SearchDef>,
    cmd: Cmd,
}

impl LastSearch {
    pub fn empty() -> Self {
        Self {
            last_char_search: None,
            search_def: None,
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

    pub fn set_cmd(&mut self, cmd: Cmd) {
        self.cmd = cmd;
    }
}
