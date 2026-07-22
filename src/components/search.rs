use crate::cmd::Motion;

pub struct LastSearch {
    char_search: Option<Motion>,
}

impl LastSearch {
    pub fn empty() -> Self {
        Self { char_search: None }
    }

    pub fn save_char_search(&mut self, m: Motion) {
        assert!(matches!(
            m,
            Motion::FindNextChar(_)
                | Motion::FindPrevChar(_)
                | Motion::TillNextChar(_)
                | Motion::TillPrevChar(_)
        ));
        self.char_search = Some(m);
    }

    pub fn char_search(&self) -> Option<Motion> {
        self.char_search
    }
}
