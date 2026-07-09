use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secondary {
    HardQuit,
    CondWriteAndQuit,
    GotoLine,
    Char(char),
}

impl Secondary {
    pub fn from(event: KeyEvent) -> Option<Self> {
        match event.code {
            KeyCode::Char('Q') => Some(Secondary::HardQuit),
            KeyCode::Char('Z') => Some(Secondary::CondWriteAndQuit),
            KeyCode::Char('g') => Some(Secondary::GotoLine),
            _ => None,
        }
    }
}
