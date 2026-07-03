use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secondary {
    HardQuit,
    CondWriteAndQuit,
}

impl Secondary {
    pub fn from(event: KeyEvent) -> Option<Self> {
        match event.code {
            KeyCode::Char('Q') => Some(Secondary::HardQuit),
            KeyCode::Char('Z') => Some(Secondary::CondWriteAndQuit),
            _ => None,
        }
    }
}
