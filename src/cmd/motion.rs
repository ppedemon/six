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
    EndBigWord,
    EndSubWord,
    FirstNonBlankInLine,
    StartOfLine,
    EndOfLine,
    FirstNonBlankInFile,
    StartOfFile,
    EndOfFile,
}

impl Motion {
    pub fn from(event: KeyEvent) -> Option<Self> {
        match event.code {
            KeyCode::Char('j') | KeyCode::Down => Some(Motion::Down),
            KeyCode::Char('k') | KeyCode::Up => Some(Motion::Up),
            KeyCode::Char('h') | KeyCode::Left => Some(Motion::Left),
            KeyCode::Char('l') | KeyCode::Right => Some(Motion::Right),

            KeyCode::Char('u') if event.modifiers == KeyModifiers::CONTROL => Some(Motion::PageUp),
            KeyCode::Char('d') if event.modifiers == KeyModifiers::CONTROL => {
                Some(Motion::PageDown)
            }
            KeyCode::PageUp => Some(Motion::PageUp),
            KeyCode::PageDown => Some(Motion::PageDown),

            KeyCode::Char('W') => Some(Motion::NextBigWord),
            KeyCode::Char('w') => Some(Motion::NextSubWord),
            KeyCode::Char('B') => Some(Motion::PrevBigWord),
            KeyCode::Char('b') => Some(Motion::PrevSubWord),
            KeyCode::Char('E') => Some(Motion::EndBigWord),
            KeyCode::Char('e') => Some(Motion::EndSubWord),

            KeyCode::Char('0') => Some(Motion::StartOfLine),
            KeyCode::Char('^') => Some(Motion::FirstNonBlankInLine),
            KeyCode::Char('$') => Some(Motion::EndOfLine),
            KeyCode::Home => {
                if event.modifiers == KeyModifiers::CONTROL {
                    Some(Motion::FirstNonBlankInFile)
                } else {
                    Some(Motion::StartOfLine)
                }
            }
            KeyCode::End => {
                if event.modifiers == KeyModifiers::CONTROL {
                    Some(Motion::EndOfFile)
                } else {
                    Some(Motion::EndOfLine)
                }
            }
            _ => None,
        }
    }
}
