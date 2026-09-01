#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    // Basic movements
    Down,
    Up,
    Left,
    Right,
    PageDown,
    PageUp,

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
