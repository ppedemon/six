use crossterm::event::KeyEvent;
use std::sync::LazyLock;

use crate::{
    cmd::{
        ExMode, ImmediateOp, InsertPoint, InteractiveOp, Motion, Operator, SysOp, TextObjectKind,
        TextObjectScope,
    },
    systems::input::{
        evt::*,
        trie::{FindResult, Trie},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpResult {
    pub op: Operator,
    pub needs_arg: bool,
}

fn op<T: Into<Operator>>(t: T) -> OpResult {
    OpResult {
        op: t.into(),
        needs_arg: false,
    }
}

fn needy_op(op: Operator) -> OpResult {
    OpResult {
        op,
        needs_arg: true,
    }
}

static OP_TRIE: LazyLock<Trie<KeyEvent, OpResult>> = LazyLock::new(|| {
    let mut t = Trie::new();
    t.insert(&[char('i')], op(SysOp::EnterInsert(InsertPoint::Curr)));
    t.insert(&[char('I')], op(SysOp::EnterInsert(InsertPoint::First)));
    t.insert(&[char('a')], op(SysOp::EnterInsert(InsertPoint::Next)));
    t.insert(&[char('A')], op(SysOp::EnterInsert(InsertPoint::Last)));
    t.insert(&[char(':')], op(SysOp::EnterEx(ExMode::Colon)));
    t.insert(&[char('/')], op(SysOp::EnterEx(ExMode::SearchForward)));
    t.insert(&[char('/')], op(SysOp::EnterEx(ExMode::SearchForward)));
    t.insert(&[char('Z'), char('Z')], op(SysOp::CondWriteAndQuit));
    t.insert(&[char('Z'), char('Q')], op(SysOp::HardQuit));

    t.insert(&[char('O')], op(InteractiveOp::OpenAbove));
    t.insert(&[char('o')], op(InteractiveOp::OpenBelow));

    t.insert(&[char('x')], op(ImmediateOp::DeleteChar));
    t
});

enum MotionResult {
    Motion(Motion),
    WantsReps(fn(Option<usize>) -> Motion),
    WantsCharArg(fn(char) -> Motion),
}

fn motion(motion: Motion) -> MotionResult {
    MotionResult::Motion(motion)
}

fn wants_reps(f: fn(Option<usize>) -> Motion) -> MotionResult {
    MotionResult::WantsReps(f)
}

fn wants_char_arg(f: fn(char) -> Motion) -> MotionResult {
    MotionResult::WantsCharArg(f)
}

static MOTION_TRIE: LazyLock<Trie<KeyEvent, MotionResult>> = LazyLock::new(|| {
    let mut t = Trie::new();
    t.insert(&[char('j')], motion(Motion::Down));
    t.insert(&[down()], motion(Motion::Down));
    t.insert(&[char('k')], motion(Motion::Up));
    t.insert(&[up()], motion(Motion::Up));
    t.insert(&[char('h')], motion(Motion::Left));
    t.insert(&[left()], motion(Motion::Left));
    t.insert(&[char('l')], motion(Motion::Right));
    t.insert(&[right()], motion(Motion::Right));
    t.insert(&[char('d').ctrl()], motion(Motion::PageDown));
    t.insert(&[pg_down()], motion(Motion::PageDown));
    t.insert(&[char('u').ctrl()], motion(Motion::PageUp));
    t.insert(&[pg_up()], motion(Motion::PageUp));

    t.insert(&[char('W')], motion(Motion::NextBigWord));
    t.insert(&[char('w')], motion(Motion::NextSubWord));
    t.insert(&[char('B')], motion(Motion::PrevBigWord));
    t.insert(&[char('b')], motion(Motion::PrevSubWord));
    t.insert(&[char('E')], motion(Motion::EndBigWord));
    t.insert(&[char('e')], motion(Motion::EndSubWord));

    t.insert(&[char('f')], wants_char_arg(Motion::FindNextChar));
    t.insert(&[char('F')], wants_char_arg(Motion::FindPrevChar));
    t.insert(&[char('t')], wants_char_arg(Motion::TillNextChar));
    t.insert(&[char('T')], wants_char_arg(Motion::FindPrevChar));
    t.insert(&[char(';')], motion(Motion::RepeatForward));
    t.insert(&[char(',')], motion(Motion::RepeatBackward));

    t.insert(&[char('0')], motion(Motion::StartOfLine));
    t.insert(&[char('^')], motion(Motion::FirstNonBlankInLine));
    t.insert(&[char('$')], motion(Motion::EndOfLine));
    t.insert(&[home().ctrl()], motion(Motion::FirstNonBlankInFile));
    t.insert(&[home()], motion(Motion::StartOfLine));
    t.insert(&[end().ctrl()], motion(Motion::EndOfFile));
    t.insert(&[end()], motion(Motion::EndOfLine));

    t.insert(
        &[char('G')],
        wants_reps(|opt_line| Motion::GotoLine(opt_line.unwrap_or(usize::MAX))),
    );
    t.insert(
        &[char('g'), char('g')],
        wants_reps(|opt_line| Motion::GotoLine(opt_line.unwrap_or(1))),
    );

    t
});

fn parse_motion(reps: Option<usize>, input: &[KeyEvent]) -> FindResult<Motion> {
    // First, let's see if we have a motion expecting an arg
    if input.len() > 1 {
        let (s, args) = input.split_at(input.len() - 1);
        let arg = args[0];
        match MOTION_TRIE.find(s) {
            FindResult::Hit(MotionResult::WantsCharArg(f)) => {
                if let Some(c) = arg.code.as_char() {
                    return FindResult::Hit(f(c));
                }
            }
            _ => {}
        }
    }

    // If not, fall back to the usual parsing
    match MOTION_TRIE.find(input) {
        FindResult::Miss => FindResult::Miss,
        FindResult::Partial => FindResult::Partial,
        FindResult::Hit(MotionResult::Motion(m)) => FindResult::Hit(*m),
        FindResult::Hit(MotionResult::WantsReps(f)) => FindResult::Hit(f(reps)),
        FindResult::Hit(MotionResult::WantsCharArg(_)) => FindResult::Partial,
    }
}

pub fn parse_motion_arg(
    op: Operator,
    reps: Option<usize>,
    input: &[KeyEvent],
) -> FindResult<Motion> {
    if let Some(c) = op.line_arg_char()
        && input.len() == 1
        && input[0].code.as_char().is_some_and(|arg| arg == c)
    {
        return FindResult::Hit(Motion::Line);
    }
    parse_motion(reps, input)
}

pub fn parse_op(reps: Option<usize>, input: &[KeyEvent]) -> FindResult<OpResult> {
    match OP_TRIE.find(input) {
        FindResult::Miss => match parse_motion(reps, input) {
            FindResult::Miss => FindResult::Miss,
            FindResult::Partial => FindResult::Partial,
            FindResult::Hit(m) => FindResult::Hit(op(m)),
        },
        FindResult::Partial => FindResult::Partial,
        FindResult::Hit(&op_result) => FindResult::Hit(op_result),
    }
}

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
