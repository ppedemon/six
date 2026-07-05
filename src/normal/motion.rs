use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Down,
    Up,
    Left,
    Right,
    PageDown,
    PageUp,
    NextBigWord,
    NextSubWord,
    PrevBigWord,
    PrevSubWord,
}

fn only_shit(modifiers: KeyModifiers) -> bool {
    modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT
}

impl Motion {
    pub fn from(event: KeyEvent) -> Option<Self> {
        if only_shit(event.modifiers) {
            match event.code {
                KeyCode::Char('j') | KeyCode::Down => Some(Motion::Down),
                KeyCode::Char('k') | KeyCode::Up => Some(Motion::Up),
                KeyCode::Char('h') | KeyCode::Left => Some(Motion::Left),
                KeyCode::Char('l') | KeyCode::Right => Some(Motion::Right),
                KeyCode::PageUp => Some(Motion::PageUp),
                KeyCode::PageDown => Some(Motion::PageDown),
                KeyCode::Char('W') => Some(Motion::NextBigWord),
                KeyCode::Char('w') => Some(Motion::NextSubWord),
                KeyCode::Char('B') => Some(Motion::PrevBigWord),
                KeyCode::Char('b') => Some(Motion::PrevSubWord),
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
