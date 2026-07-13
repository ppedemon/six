use crossterm::event::{KeyCode, KeyEvent};

use crate::cmd::Motion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOp {
    FindNextChar,
    FindPrevChar,
    TillNextChar,
    TillPrevChar,
    RepeatForward,
    RepeatBackward,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysOp {
    EnterNormal,
    EnterEx(ExMode),
    EnterInsert(InsertPoint),
    OpenAbove,
    OpenBelow,
    BufferOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Nop,
    Sys(SysOp),
    Move(Motion),
    Search(SearchOp),
}

impl From<SysOp> for Operator {
    fn from(sys_op: SysOp) -> Self {
        Self::Sys(sys_op)
    }
}

impl From<Motion> for Operator {
    fn from(motion: Motion) -> Self {
        Self::Move(motion)
    }
}

impl From<SearchOp> for Operator {
    fn from(find_op: SearchOp) -> Self {
        Self::Search(find_op)
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
            KeyCode::Char('O') => Some(SysOp::OpenAbove.into()),
            KeyCode::Char('o') => Some(SysOp::OpenBelow.into()),
            KeyCode::Char('Z') => Some(SysOp::BufferOp.into()),

            KeyCode::Char('f') => Some(SearchOp::FindNextChar.into()),
            KeyCode::Char('F') => Some(SearchOp::FindPrevChar.into()),
            KeyCode::Char('t') => Some(SearchOp::TillNextChar.into()),
            KeyCode::Char('T') => Some(SearchOp::TillPrevChar.into()),
            KeyCode::Char(';') => Some(SearchOp::RepeatForward.into()),
            KeyCode::Char(',') => Some(SearchOp::RepeatBackward.into()),

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
