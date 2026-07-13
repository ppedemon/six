use crossterm::event::{KeyCode, KeyEvent};

use crate::cmd::Motion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertPoint {
    Curr,
    Next,
    First,
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExMode {
    Colon,
    SearchForward,
    SearchBackward,
}

// System commands:
// Mode switching, general buffer ops (like ZZ and ZQ),
// TODO add window management here
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysOp {
    EnterNormal,
    EnterEx(ExMode),
    EnterInsert(InsertPoint),
    BufferOp,
}

// Search operations:
// Find next/prev chars, keep state.
// TODO add regexp searches here
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOp {
    FindNextChar,
    FindPrevChar,
    TillNextChar,
    TillPrevChar,
    RepeatForward,
    RepeatBackward,
}

// Interactive commands:
// Do something, then enter insert mode. For example, o,O,c
// TODO add c,C,s
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractiveOp {
    OpenAbove,
    OpenBelow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Nop,
    Move(Motion),
    Sys(SysOp),
    Search(SearchOp),
    Interactive(InteractiveOp),
}

impl From<Motion> for Operator {
    fn from(motion: Motion) -> Self {
        Self::Move(motion)
    }
}

impl From<SysOp> for Operator {
    fn from(sys_op: SysOp) -> Self {
        Self::Sys(sys_op)
    }
}

impl From<SearchOp> for Operator {
    fn from(find_op: SearchOp) -> Self {
        Self::Search(find_op)
    }
}

impl From<InteractiveOp> for Operator {
    fn from(interactive_op: InteractiveOp) -> Self {
        Self::Interactive(interactive_op)
    }
}

impl Operator {
    pub fn from(event: KeyEvent) -> Option<Operator> {
        match event.code {
            KeyCode::Char('i') => Some(SysOp::EnterInsert(InsertPoint::Curr).into()),
            KeyCode::Char('I') => Some(SysOp::EnterInsert(InsertPoint::First).into()),
            KeyCode::Char('a') => Some(SysOp::EnterInsert(InsertPoint::Next).into()),
            KeyCode::Char('A') => Some(SysOp::EnterInsert(InsertPoint::Last).into()),
            KeyCode::Char(':') => Some(SysOp::EnterEx(ExMode::Colon).into()),
            KeyCode::Char('/') => Some(SysOp::EnterEx(ExMode::SearchForward).into()),
            KeyCode::Char('?') => Some(SysOp::EnterEx(ExMode::SearchBackward).into()),
            KeyCode::Char('Z') => Some(SysOp::BufferOp.into()),

            KeyCode::Char('f') => Some(SearchOp::FindNextChar.into()),
            KeyCode::Char('F') => Some(SearchOp::FindPrevChar.into()),
            KeyCode::Char('t') => Some(SearchOp::TillNextChar.into()),
            KeyCode::Char('T') => Some(SearchOp::TillPrevChar.into()),
            KeyCode::Char(';') => Some(SearchOp::RepeatForward.into()),
            KeyCode::Char(',') => Some(SearchOp::RepeatBackward.into()),

            KeyCode::Char('O') => Some(InteractiveOp::OpenAbove.into()),
            KeyCode::Char('o') => Some(InteractiveOp::OpenBelow.into()),

            _ => Motion::from(event).map(Operator::Move),
        }
    }

    pub fn needs_arg(&self) -> bool {
        match self {
            Self::Sys(SysOp::BufferOp) => true,
            Self::Search(SearchOp::FindNextChar) => true,
            Self::Search(SearchOp::FindPrevChar) => true,
            Self::Search(SearchOp::TillNextChar) => true,
            Self::Search(SearchOp::TillPrevChar) => true,
            Self::Move(Motion::SmallGotoLine) => true,
            _ => false,
        }
    }
}
