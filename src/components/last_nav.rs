use crate::cmd::Motion;

pub struct LastNav {
    overshot: bool,
    last_char_search: Option<Motion>,
}

impl LastNav {
    pub fn empty() -> Self {
        Self {
            overshot: false,
            last_char_search: None,
        }
    }

    pub fn clear_overshot(&mut self) {
        self.overshot = false;
    }

    pub fn save_overshot(&mut self) {
        self.overshot = true;
    }

    pub fn last_nav_overshot(&self) -> bool {
        self.overshot
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
}
