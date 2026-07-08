use crossterm::event::{KeyCode, KeyEvent};

use crate::cmd::Motion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOp {
    InsertChar(char),
    Tab,
    Enter,
    Backspace,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOp {
    FindNextChar,
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
    BufferOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Nop,
    Sys(SysOp),
    Move(Motion),
    Find(SearchOp),
    Edit(EditOp),
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
        Self::Find(find_op)
    }
}

impl From<EditOp> for Operator {
    fn from(edit_op: EditOp) -> Self {
        Self::Edit(edit_op)
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

            _ => Motion::from(event).map(Operator::Move),
        }
    }

    pub fn needs_target(&self) -> bool {
        match self {
            Self::Sys(SysOp::BufferOp) => true,
            Self::Find(_) => true,
            _ => false,
        }
    }
}
