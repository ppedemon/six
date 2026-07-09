use crate::cmd::SearchOp;

pub struct CharSearch {
    pub c: char,
    pub op: SearchOp,
}

pub struct LastSearch {
    pub char_search: Option<CharSearch>,
}

impl LastSearch {
    pub fn empty() -> Self {
        Self {
            char_search: None,
        }
    }

    pub fn set_char(&mut self, c: char, op: SearchOp) {
        self.char_search = Some(CharSearch { c, op });
    }
}
