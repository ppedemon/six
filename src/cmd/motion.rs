#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    // Basic movements
    Down,
    Up,
    Left,
    Right,
    PageDown,
    PageUp,
    GotoCol(usize),

    // Jump between words
    NextBigWord,
    NextSubWord,
    PrevBigWord,
    PrevSubWord,
    EndBigWord,
    EndSubWord,

    // Char-based in current line
    FindNextChar(char),
    FindPrevChar(char),
    TillNextChar(char),
    TillPrevChar(char),
    RepeatForward,
    RepeatBackward,

    // Line-based
    Line,
    GotoLine(usize), // usize::MAX = last line
    FirstNonBlankInLine,
    StartOfLine,
    EndOfLine,
    EndOfLinePlusOne, // Non-parseable, only for internal use

    // File-based
    FirstNonBlankInFile,
    StartOfFile,
    EndOfFile,

    // Mark-based
    GotoMark(char),
    ExactGotoMark(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionMeta {
    Viewport,
    Charwise,
    Linewise,
}

// -----------------------------------------------------------------------
// Movement metadata.
// We do out best to follow what the vim reference manual says.
// See (in vim) :help motions
// -----------------------------------------------------------------------
impl Motion {
    pub fn meta(&self) -> MotionMeta {
        match self {
            Self::Down => MotionMeta::Linewise,
            Self::Up => MotionMeta::Linewise,
            Self::Left => MotionMeta::Charwise,
            Self::Right => MotionMeta::Charwise,
            Self::PageDown => MotionMeta::Viewport,
            Self::PageUp => MotionMeta::Viewport,
            Self::GotoCol(_) => MotionMeta::Charwise,

            Self::NextBigWord => MotionMeta::Charwise,
            Self::NextSubWord => MotionMeta::Charwise,
            Self::PrevBigWord => MotionMeta::Charwise,
            Self::PrevSubWord => MotionMeta::Charwise,
            Self::EndBigWord => MotionMeta::Charwise,
            Self::EndSubWord => MotionMeta::Charwise,

            Self::FindNextChar(char) => MotionMeta::Charwise,
            Self::FindPrevChar(char) => MotionMeta::Charwise,
            Self::TillNextChar(char) => MotionMeta::Charwise,
            Self::TillPrevChar(char) => MotionMeta::Charwise,
            Self::RepeatForward => MotionMeta::Charwise,
            Self::RepeatBackward => MotionMeta::Charwise,

            Self::Line => MotionMeta::Linewise,
            Self::GotoLine(_) => MotionMeta::Linewise,
            Self::FirstNonBlankInLine => MotionMeta::Charwise,
            Self::StartOfLine => MotionMeta::Charwise,
            Self::EndOfLine => MotionMeta::Charwise,
            Self::EndOfLinePlusOne => MotionMeta::Charwise,

            Self::FirstNonBlankInFile => MotionMeta::Linewise,
            Self::StartOfFile => MotionMeta::Linewise,
            Self::EndOfFile => MotionMeta::Linewise,

            Self::GotoMark(char) => MotionMeta::Linewise,
            Self::ExactGotoMark(char) => MotionMeta::Charwise,
        }
    }
}
