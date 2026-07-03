use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Down,
    Up,
    Left,
    Right,
    PageDown,
    PageUp,
}

impl Motion {
    pub fn from(event: KeyEvent) -> Option<Self> {
        if event.modifiers.is_empty() {
            match event.code {
                KeyCode::Char('j') | KeyCode::Down => Some(Motion::Down),
                KeyCode::Char('k') | KeyCode::Up => Some(Motion::Up),
                KeyCode::Char('h') | KeyCode::Left => Some(Motion::Left),
                KeyCode::Char('l') | KeyCode::Right => Some(Motion::Right),
                KeyCode::PageUp => Some(Motion::PageUp),
                KeyCode::PageDown => Some(Motion::PageDown),
                _ => None,
            }
        } else if event.modifiers == KeyModifiers::CONTROL {
            match event.code {
                KeyCode::Char('u') => Some(Motion::PageUp),
                KeyCode::Char('d') => Some(Motion::PageDown),
                _ => None,
            }
        } else {
            None
        }
    }
}
