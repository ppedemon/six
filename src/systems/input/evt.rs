use std::fmt::{self, Write};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn char(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty())
}

pub fn tab() -> KeyEvent {
    KeyEvent::new(KeyCode::Tab, KeyModifiers::empty())
}

pub fn enter() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
}

pub fn backspace() -> KeyEvent {
    KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty())
}

pub fn delete() -> KeyEvent {
    KeyEvent::new(KeyCode::Delete, KeyModifiers::empty())
}

pub fn left() -> KeyEvent {
    KeyEvent::new(KeyCode::Left, KeyModifiers::empty())
}

pub fn right() -> KeyEvent {
    KeyEvent::new(KeyCode::Right, KeyModifiers::empty())
}

pub fn up() -> KeyEvent {
    KeyEvent::new(KeyCode::Up, KeyModifiers::empty())
}

pub fn down() -> KeyEvent {
    KeyEvent::new(KeyCode::Down, KeyModifiers::empty())
}

pub fn pg_up() -> KeyEvent {
    KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty())
}

pub fn pg_down() -> KeyEvent {
    KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty())
}

pub fn home() -> KeyEvent {
    KeyEvent::new(KeyCode::Home, KeyModifiers::empty())
}

pub fn end() -> KeyEvent {
    KeyEvent::new(KeyCode::End, KeyModifiers::empty())
}

pub trait PimpKeyEvent {
    fn ctrl(self) -> Self;
    fn alt(self) -> Self;
}

impl PimpKeyEvent for KeyEvent {
    fn ctrl(mut self) -> Self {
        self.modifiers |= KeyModifiers::CONTROL;
        self
    }

    fn alt(mut self) -> Self {
        self.modifiers |= KeyModifiers::ALT;
        self
    }
}

pub struct PrettyEvent<'a>(&'a KeyEvent);

pub trait Pretty {
    fn pretty(&self) -> PrettyEvent<'_>;
}

impl Pretty for KeyEvent {
    fn pretty(&self) -> PrettyEvent<'_> {
        PrettyEvent(self)
    }
}

impl fmt::Display for PrettyEvent<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.0.modifiers {
            m if m.contains(KeyModifiers::CONTROL | KeyModifiers::ALT) => "<C-M-",
            m if m.contains(KeyModifiers::CONTROL) => "<C-",
            m if m.contains(KeyModifiers::ALT) => "<M-",
            _ => "",
        };
        f.write_str(prefix)?;

        match self.0.code {
            KeyCode::Char(c) => f.write_char(c)?,
            KeyCode::Tab => f.write_str(r"\t")?,
            KeyCode::Enter => f.write_str(r"\n")?,
            KeyCode::Backspace => f.write_str("Backspace")?,
            KeyCode::Delete => f.write_str("Del")?,
            KeyCode::Up => f.write_str("Up")?,
            KeyCode::Down => f.write_str("Down")?,
            KeyCode::Left => f.write_str("Left")?,
            KeyCode::Right => f.write_str("Right")?,
            KeyCode::PageUp => f.write_str("PgUp")?,
            KeyCode::PageDown => f.write_str("PgDown")?,
            KeyCode::Home => f.write_str("Home")?,
            KeyCode::End => f.write_str("End")?,
            KeyCode::Esc => f.write_str("Esc")?,
            KeyCode::CapsLock => f.write_str("CapsLk")?,
            KeyCode::NumLock => f.write_str("NumLk")?,
            KeyCode::ScrollLock => f.write_str("ScrLk")?,
            KeyCode::PrintScreen => f.write_str("PrtSc")?,
            KeyCode::Insert => f.write_str("Ins")?,
            KeyCode::F(n) => write!(f, "F{}", n)?,
            _ => f.write_str("??")?,
        }

        if !prefix.is_empty() {
            f.write_char('>')?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic() {
        let evt = pg_up();
        assert_eq!(evt.code, KeyCode::PageUp);
        assert!(evt.modifiers.is_empty());
    }

    #[test]
    fn test_ctrl_pimp() {
        let evt = char('d').ctrl();
        assert_eq!(evt.code.as_char().unwrap(), 'd');
        assert!(evt.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn test_alt_pimp() {
        let evt = char('w').alt();
        assert_eq!(evt.code.as_char().unwrap(), 'w');
        assert!(evt.modifiers.contains(KeyModifiers::ALT));
    }
}
