use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::cmd::{
    ExMode, ImmediateOp, InsertPoint, InteractiveOp, Motion, Operator, SysOp, TextObjectKind,
    TextObjectScope,
};

pub fn parse_digit(evt: KeyEvent) -> Option<u32> {
    if evt.modifiers.is_empty() {
        evt.code.as_char().and_then(|c| c.to_digit(10))
    } else {
        None
    }
}

pub fn parse_non_zero_digit(evt: KeyEvent) -> Option<u32> {
    parse_digit(evt).and_then(|d| if d > 0 { Some(d) } else { None })
}

pub fn starts_reg(evt: KeyEvent) -> bool {
    evt.code.as_char().is_some_and(|c| c == '"')
}

pub fn parse_reg(evt: KeyEvent) -> Option<char> {
    evt.code.as_char().and_then(|c| match c {
        _ if c.is_ascii_digit() => Some(c),
        _ if c.is_ascii_alphabetic() => Some(c),
        _ if "%#.:/=-_".contains(c) => Some(c),
        _ => None,
    })
}

pub enum ParseResult<T> {
    Error,
    Cont,
    Done { result: T, needs_args: bool },
}

fn err<T>() -> ParseResult<T> {
    ParseResult::Error
}

fn cont<T>(input: &mut String, c: char) -> ParseResult<T> {
    input.push(c);
    ParseResult::Cont
}

fn motion(m: Motion) -> ParseResult<Motion> {
    ParseResult::Done {
        result: m,
        needs_args: false,
    }
}

fn op(op: Operator) -> ParseResult<Operator> {
    ParseResult::Done {
        result: op,
        needs_args: false,
    }
}

fn needy_op(op: Operator) -> ParseResult<Operator> {
    ParseResult::Done {
        result: op,
        needs_args: true,
    }
}

fn parse_motion(reps: Option<usize>, input: &mut String, evt: KeyEvent) -> ParseResult<Motion> {
    if input.is_empty() {
        match evt.code {
            KeyCode::Char('j') | KeyCode::Down => motion(Motion::Down),
            KeyCode::Char('k') | KeyCode::Up => motion(Motion::Up),
            KeyCode::Char('h') | KeyCode::Left => motion(Motion::Left),
            KeyCode::Char('l') | KeyCode::Right => motion(Motion::Right),
            KeyCode::Char('u') if evt.modifiers == KeyModifiers::CONTROL => motion(Motion::PageUp),
            KeyCode::Char('d') if evt.modifiers == KeyModifiers::CONTROL => {
                motion(Motion::PageDown)
            }
            KeyCode::PageUp => motion(Motion::PageUp),
            KeyCode::PageDown => motion(Motion::PageDown),

            KeyCode::Char('W') => motion(Motion::NextBigWord),
            KeyCode::Char('w') => motion(Motion::NextSubWord),
            KeyCode::Char('B') => motion(Motion::PrevBigWord),
            KeyCode::Char('b') => motion(Motion::PrevSubWord),
            KeyCode::Char('E') => motion(Motion::EndBigWord),
            KeyCode::Char('e') => motion(Motion::EndSubWord),

            KeyCode::Char('f') => cont(input, 'f'),
            KeyCode::Char('F') => cont(input, 'F'),
            KeyCode::Char('t') => cont(input, 't'),
            KeyCode::Char('T') => cont(input, 'T'),
            KeyCode::Char(';') => motion(Motion::RepeatForward),
            KeyCode::Char(',') => motion(Motion::RepeatBackward),

            KeyCode::Char('0') => motion(Motion::StartOfLine),
            KeyCode::Char('^') => motion(Motion::FirstNonBlankInLine),
            KeyCode::Char('$') => motion(Motion::EndOfLine),
            KeyCode::Home => {
                if evt.modifiers == KeyModifiers::CONTROL {
                    motion(Motion::FirstNonBlankInFile)
                } else {
                    motion(Motion::StartOfLine)
                }
            }
            KeyCode::End => {
                if evt.modifiers == KeyModifiers::CONTROL {
                    motion(Motion::EndOfFile)
                } else {
                    motion(Motion::EndOfLine)
                }
            }

            KeyCode::Char('G') => motion(Motion::GotoLine(reps.unwrap_or(usize::MAX))),
            KeyCode::Char('g') => cont(input, 'g'),
            _ => err(),
        }
    } else {
        match (input.as_str(), evt.code) {
            ("g", KeyCode::Char(g)) => motion(Motion::GotoLine(reps.unwrap_or(1))),
            ("f", KeyCode::Char(c)) => motion(Motion::FindNextChar(c)),
            ("F", KeyCode::Char(c)) => motion(Motion::FindPrevChar(c)),
            ("t", KeyCode::Char(c)) => motion(Motion::TillNextChar(c)),
            ("T", KeyCode::Char(c)) => motion(Motion::TillPrevChar(c)),
            _ => err(),
        }
    }
}

pub fn parse_motion_arg(
    op: Operator,
    reps: Option<usize>,
    input: &mut String,
    evt: KeyEvent,
) -> ParseResult<Motion> {
    if let Some(c) = op.line_arg_char()
        && evt.code.as_char().is_some_and(|op_c| op_c == c)
    {
        motion(Motion::Line)
    } else {
        parse_motion(reps, input, evt)
    }
}

pub fn parse_op(reps: Option<usize>, input: &mut String, evt: KeyEvent) -> ParseResult<Operator> {
    if input.is_empty() {
        match evt.code {
            KeyCode::Char('i') => op(SysOp::EnterInsert(InsertPoint::Curr).into()),
            KeyCode::Char('I') => op(SysOp::EnterInsert(InsertPoint::First).into()),
            KeyCode::Char('a') => op(SysOp::EnterInsert(InsertPoint::Next).into()),
            KeyCode::Char('A') => op(SysOp::EnterInsert(InsertPoint::Last).into()),
            KeyCode::Char(':') => op(SysOp::EnterEx(ExMode::Colon).into()),
            KeyCode::Char('/') => op(SysOp::EnterEx(ExMode::SearchForward).into()),
            KeyCode::Char('?') => op(SysOp::EnterEx(ExMode::SearchBackward).into()),
            KeyCode::Char('Z') => cont(input, 'Z'),

            KeyCode::Char('O') => op(InteractiveOp::OpenAbove.into()),
            KeyCode::Char('o') => op(InteractiveOp::OpenBelow.into()),

            KeyCode::Char('x') => op(ImmediateOp::DeleteChar.into()),

            _ => match parse_motion(reps, input, evt) {
                ParseResult::Error => ParseResult::Error,
                ParseResult::Cont => ParseResult::Cont,
                ParseResult::Done { result, .. } => op(result.into()),
            },
        }
    } else {
        match (input.as_str(), evt.code) {
            ("Z", KeyCode::Char('Q')) => op(SysOp::HardQuit.into()),
            ("Z", KeyCode::Char('Z')) => op(SysOp::CondWriteAndQuit.into()),
            _ => match parse_motion(reps, input, evt) {
                ParseResult::Error => ParseResult::Error,
                ParseResult::Cont => ParseResult::Cont,
                ParseResult::Done { result, .. } => op(result.into()),
            },
        }
    }
}

pub fn parse_textobject_scope(evt: KeyEvent) -> Option<TextObjectScope> {
    evt.code.as_char().and_then(|c| match c {
        'i' => Some(TextObjectScope::Inside),
        'a' => Some(TextObjectScope::Around),
        _ => None,
    })
}

pub fn parse_textobject_kind(evt: KeyEvent) -> Option<TextObjectKind> {
    evt.code.as_char().and_then(|c| match c {
        '"' => Some(TextObjectKind::DoubleQuote),
        '\'' => Some(TextObjectKind::SingleQuote),
        '(' | ')' => Some(TextObjectKind::Paren),
        '[' | ']' => Some(TextObjectKind::Bracket),
        '{' | '}' => Some(TextObjectKind::Brace),
        'w' => Some(TextObjectKind::Word),
        's' => Some(TextObjectKind::Sentence),
        _ => None,
    })
}
