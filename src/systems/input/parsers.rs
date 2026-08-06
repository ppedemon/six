use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::LazyLock;

use crate::{
    cmd::{
        ExMode, ImmediateOp, InsertPoint, InteractiveOp, Motion, MotionMode, Operator, SysOp,
        TextObjectKind, TextObjectScope,
    },
    systems::input::{
        evt::*,
        trie::{FindResult, Trie},
    },
};

// Generic parse result.
// This type is what we are going to retrieve from the motion and op tries,
// telling us what to do once we hit something in the trie. One of:
//
//    - Ok(T): congrats, you just found an motion or an operator, nothing to do
//    - WantsReps(f): this is an op or motion that needs the number of reps as an arg (like G or gg)
//    - WantArgs: this is an op or motion taking an arg (think of f{char} or m{char})
#[derive(Debug, Clone, Copy)]
enum ParseResult<T> {
    Ok(T),
    WantsReps(fn(Option<usize>) -> T),
    WantsArg(fn(KeyEvent) -> Option<T>),
}

fn ok<T>(t: T) -> ParseResult<T> {
    ParseResult::Ok(t)
}

fn wants_reps<T>(f: fn(Option<usize>) -> T) -> ParseResult<T> {
    ParseResult::WantsReps(f)
}

fn wants_arg<T>(f: fn(KeyEvent) -> Option<T>) -> ParseResult<T> {
    ParseResult::WantsArg(f)
}

// When parsing operators, the T in ParseResult<T> will be an OpSpec.
// This types tells the state machine whether the returned operator
// wants an argument (a motion or a text object).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpSpec {
    pub op: Operator,
    pub needs_arg: bool,
}

fn op<T: Into<Operator>>(t: T) -> OpSpec {
    OpSpec {
        op: t.into(),
        needs_arg: false,
    }
}

fn needy_op<T: Into<Operator>>(t: T) -> OpSpec {
    OpSpec {
        op: t.into(),
        needs_arg: true,
    }
}

fn ok_op<T: Into<Operator>>(t: T) -> ParseResult<OpSpec> {
    ok(op(t))
}

fn ok_needy_op<T: Into<Operator>>(t: T) -> ParseResult<OpSpec> {
    ok(needy_op(t))
}

static OP_TRIE: LazyLock<Trie<KeyEvent, ParseResult<OpSpec>>> = LazyLock::new(|| {
    let mut t = Trie::new();
    t.insert(&[char('i')], ok_op(SysOp::EnterInsert(InsertPoint::Curr)));
    t.insert(&[char('I')], ok_op(SysOp::EnterInsert(InsertPoint::First)));
    t.insert(&[char('a')], ok_op(SysOp::EnterInsert(InsertPoint::Next)));
    t.insert(&[char('A')], ok_op(SysOp::EnterInsert(InsertPoint::Last)));
    t.insert(&[char(':')], ok_op(SysOp::EnterEx(ExMode::Colon)));
    t.insert(&[char('/')], ok_op(SysOp::EnterEx(ExMode::SearchForward)));
    t.insert(&[char('/')], ok_op(SysOp::EnterEx(ExMode::SearchForward)));
    t.insert(&[char('Z'), char('Z')], ok_op(SysOp::CondWriteAndQuit));
    t.insert(&[char('Z'), char('Q')], ok_op(SysOp::HardQuit));
    t.insert(
        &[char('m')],
        wants_arg(|evt| parse_mark(evt).map(|c| op(SysOp::AddLocalMark(c)))),
    );

    t.insert(&[char('O')], ok_op(InteractiveOp::OpenAbove));
    t.insert(&[char('o')], ok_op(InteractiveOp::OpenBelow));

    t.insert(&[char('x')], ok_op(ImmediateOp::Delete));
    t.insert(&[delete()], ok_op(ImmediateOp::Delete));
    t.insert(&[char('X')], ok_op(ImmediateOp::Backspace));
    t.insert(&[backspace()], ok_op(ImmediateOp::Backspace));
    t.insert(&[char('J')], ok_op(ImmediateOp::Join));
    t.insert(&[char('y')], ok_needy_op(ImmediateOp::Yank));
    t
});

static MOTION_TRIE: LazyLock<Trie<KeyEvent, ParseResult<Motion>>> = LazyLock::new(|| {
    let mut t = Trie::new();
    t.insert(&[char('j')], ok(Motion::Down));
    t.insert(&[down()], ok(Motion::Down));
    t.insert(&[char('k')], ok(Motion::Up));
    t.insert(&[up()], ok(Motion::Up));
    t.insert(&[char('h')], ok(Motion::Left));
    t.insert(&[left()], ok(Motion::Left));
    t.insert(&[char('l')], ok(Motion::Right));
    t.insert(&[right()], ok(Motion::Right));
    t.insert(&[char('d').ctrl()], ok(Motion::PageDown));
    t.insert(&[pg_down()], ok(Motion::PageDown));
    t.insert(&[char('u').ctrl()], ok(Motion::PageUp));
    t.insert(&[pg_up()], ok(Motion::PageUp));

    t.insert(&[char('W')], ok(Motion::NextBigWord));
    t.insert(&[char('w')], ok(Motion::NextSubWord));
    t.insert(&[char('B')], ok(Motion::PrevBigWord));
    t.insert(&[char('b')], ok(Motion::PrevSubWord));
    t.insert(&[char('E')], ok(Motion::EndBigWord));
    t.insert(&[char('e')], ok(Motion::EndSubWord));

    t.insert(
        &[char('f')],
        wants_arg(|evt| char_or_tab(evt).map(Motion::FindNextChar)),
    );
    t.insert(
        &[char('F')],
        wants_arg(|evt| char_or_tab(evt).map(Motion::FindPrevChar)),
    );
    t.insert(
        &[char('t')],
        wants_arg(|evt| char_or_tab(evt).map(Motion::TillNextChar)),
    );
    t.insert(
        &[char('T')],
        wants_arg(|evt| char_or_tab(evt).map(Motion::TillPrevChar)),
    );
    t.insert(&[char(';')], ok(Motion::RepeatForward));
    t.insert(&[char(',')], ok(Motion::RepeatBackward));

    t.insert(&[char('^')], ok(Motion::FirstNonBlankInLine));
    t.insert(&[char('0')], ok(Motion::StartOfLine));
    t.insert(&[home()], ok(Motion::StartOfLine));
    t.insert(&[char('$')], ok(Motion::EndOfLine));
    t.insert(&[end()], ok(Motion::EndOfLine));

    t.insert(&[home().ctrl()], ok(Motion::FirstNonBlankInFile));
    t.insert(&[end().ctrl()], ok(Motion::EndOfFile));

    t.insert(
        &[char('G')],
        wants_reps(|opt_line| Motion::GotoLine(opt_line.unwrap_or(usize::MAX))),
    );
    t.insert(
        &[char('g'), char('g')],
        wants_reps(|opt_line| Motion::GotoLine(opt_line.unwrap_or(1))),
    );

    t.insert(
        &[char('\'')],
        wants_arg(|evt| parse_mark(evt).map(Motion::GotoMark)),
    );
    t.insert(
        &[char('`')],
        wants_arg(|evt| parse_mark(evt).map(Motion::ExactGotoMark)),
    );

    t
});

// Parse a top level motion.
// Used when parsing ops, since a top level motion is an op.
fn parse_motion(reps: Option<usize>, input: &[KeyEvent]) -> FindResult<Motion> {
    // First, let's see if we have a motion expecting an arg
    if input.len() > 1 {
        let (s, args) = input.split_at(input.len() - 1);
        match MOTION_TRIE.find(s) {
            FindResult::Hit(ParseResult::WantsArg(f)) => match f(args[0]) {
                Some(m) => return FindResult::Hit(m),
                None => {}
            },
            _ => {}
        }
    }

    // If not, fall back to the usual parsing
    match MOTION_TRIE.find(input) {
        FindResult::Miss => FindResult::Miss,
        FindResult::Partial => FindResult::Partial,
        FindResult::Hit(ParseResult::Ok(m)) => FindResult::Hit(*m),
        FindResult::Hit(ParseResult::WantsReps(f)) => FindResult::Hit(f(reps)),
        FindResult::Hit(ParseResult::WantsArg(_)) => FindResult::Partial,
    }
}

// Parse a motion following an operator (that is, a motion arg).
// The state machine will call this function when the parsed operator is needy.
// That means we need to parse an arg, either a motion or a text object.
//
// Note: this function deals with line scopes (dd, yy, pp, cc) as a special case.
// The Operator type provides a `line_arg_char` function, returning Some(ch)
// if an operator acts admits a double `ch` to act on lines. For example,
// line_arg_char(Operator::Delete) = Some('d').
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

pub fn parse_op(reps: Option<usize>, input: &[KeyEvent]) -> FindResult<OpSpec> {
    // First, let's see if we have an op expecting an arg
    if input.len() > 1 {
        let (s, args) = input.split_at(input.len() - 1);
        match OP_TRIE.find(s) {
            FindResult::Hit(ParseResult::WantsArg(f)) => match f(args[0]) {
                Some(op) => return FindResult::Hit(op),
                None => {}
            },
            _ => {}
        }
    }

    match OP_TRIE.find(input) {
        FindResult::Miss => match parse_motion(reps, input) {
            FindResult::Miss => FindResult::Miss,
            FindResult::Partial => FindResult::Partial,
            FindResult::Hit(m) => FindResult::Hit(op(m)),
        },
        FindResult::Partial => FindResult::Partial,
        FindResult::Hit(ParseResult::Ok(op)) => FindResult::Hit(*op),
        FindResult::Hit(ParseResult::WantsReps(f)) => FindResult::Hit(f(reps)),
        FindResult::Hit(ParseResult::WantsArg(_)) => FindResult::Partial,
    }
}

// We are only allowed to parse digraphs if the last key event we saw matches
// any of the motions taking a digraph as argument: f, F, t, or T
pub fn digraph_allowed(input: &[KeyEvent]) -> bool {
    if input.len() > 0 {
        let last = input[input.len() - 1];
        last.modifiers == KeyModifiers::empty()
            && last
                .code
                .as_char()
                .map(|c| c.to_ascii_uppercase())
                .is_some_and(|c| c == 'F' || c == 'T')
    } else {
        false
    }
}

pub fn starts_digraph(evt: KeyEvent) -> bool {
    evt.modifiers.contains(KeyModifiers::CONTROL)
        && evt.code.as_char().is_some_and(|c| c == 'k' || c == 'K')
}

// -----------------------------------------------------------------------
// Trivial parsers and convenience functions from now on
// -----------------------------------------------------------------------

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

pub fn parse_motion_mode(evt: KeyEvent) -> Option<MotionMode> {
    match evt.code.as_char() {
        Some('v') | Some('V') if evt.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(MotionMode::Blockwise)
        }
        Some('v') => Some(MotionMode::Charwise),
        Some('V') => Some(MotionMode::Linewise),
        _ => None,
    }
}

fn parse_mark(evt: KeyEvent) -> Option<char> {
    evt.code.as_char().filter(char::is_ascii_lowercase)
}

fn char_or_tab(evt: KeyEvent) -> Option<char> {
    match evt.code {
        KeyCode::Char(c) => Some(c),
        KeyCode::Tab => Some('\t'),
        _ => None,
    }
}
