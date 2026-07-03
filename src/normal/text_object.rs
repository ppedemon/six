use crossterm::event::KeyEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Inside,
    Around,
}

impl Scope {
    pub fn from(event: KeyEvent) -> Option<Self> {
        if event.modifiers.is_empty() {
            event.code.as_char().and_then(|c| match c {
                'i' => Some(Scope::Inside),
                'a' => Some(Scope::Around),
                _ => None,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    DoubleQuote,
    SingleQuote,
    Paren,
    Bracket,
    Brace,
    Word,
    Sentence,
}

impl Kind {
    pub fn from(event: KeyEvent) -> Option<Self> {
        if event.modifiers.is_empty() {
            event.code.as_char().and_then(|c| match c {
                '"' => Some(Kind::DoubleQuote),
                '\'' => Some(Kind::SingleQuote),
                '(' | ')' => Some(Kind::Paren),
                '[' | ']' => Some(Kind::Bracket),
                '{' | '}' => Some(Kind::Brace),
                'w' => Some(Kind::Word),
                's' => Some(Kind::Sentence),
                _ => None,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextObject {
    pub scope: Scope,
    pub kind: Kind,
}
