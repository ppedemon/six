#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextObjectScope {
    Inside,
    Around,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextObjectKind {
    DoubleQuote,
    SingleQuote,
    Paren,
    Bracket,
    Brace,
    Word,
    Sentence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextObject {
    pub scope: TextObjectScope,
    pub kind: TextObjectKind,
}

impl TextObject {
    pub fn new(scope: TextObjectScope, kind: TextObjectKind) -> Self {
        Self { scope, kind }
    }
}
