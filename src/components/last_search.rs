use crate::cmd::{Cmd, Motion, Operator};
use regex::Regex;
use std::{fmt, write};

#[derive(Debug, Clone, Copy)]
pub enum OffsetSide {
    Start,
    End,
}

impl fmt::Display for OffsetSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start => write!(f, "s"),
            Self::End => write!(f, "e"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OffsetSign {
    Plus,
    Minus,
}

impl fmt::Display for OffsetSign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Offset {
    Empty,
    Inclusive,
    Line {
        sign: OffsetSign,
        stride: usize,
    },
    Char {
        side: OffsetSide,
        sign: OffsetSign,
        stride: usize,
    },
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => Ok(()),
            Self::Inclusive => write!(f, "e"),
            Self::Line { sign, stride } => write!(f, "{sign}{stride}"),
            Self::Char { side, sign, stride } => write!(f, "{side}{sign}{stride}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Backward,
    Forward,
}

#[derive(Debug, Clone)]
pub struct PatSearch {
    pub direction: Direction,
    pub pat: Option<Regex>,
    pub offset: Offset,
}

impl fmt::Display for PatSearch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = if self.direction == Direction::Forward {
            '/'
        } else {
            '?'
        };
        write!(
            f,
            "{c}{}",
            self.pat.as_ref().map(|r| r.as_str()).unwrap_or_default()
        )?;
        match self.offset {
            Offset::Empty => Ok(()),
            other => write!(f, "{c}{other}"),
        }
    }
}

pub struct LastSearch {
    last_char_search: Option<Motion>,
    last_pat_search: Option<PatSearch>,
    curr_pat_search: Vec<PatSearch>,
    cmd: Cmd,
}

impl LastSearch {
    pub fn empty() -> Self {
        Self {
            last_char_search: None,
            last_pat_search: None,
            curr_pat_search: Vec::new(),
            cmd: Cmd::new(Operator::Nop),
        }
    }

    pub fn save_char_search(&mut self, m: Motion) {
        assert!(matches!(
            m,
            Motion::FindNextChar(_)
                | Motion::FindPrevChar(_)
                | Motion::TillNextChar(_)
                | Motion::TillPrevChar(_)
        ));
        self.last_char_search = Some(m);
    }

    pub fn last_char_search(&self) -> Option<Motion> {
        self.last_char_search
    }

    pub fn last_pat_search(&self) -> Option<&PatSearch> {
        self.last_pat_search.as_ref()
    }

    pub fn set_cmd(&mut self, cmd: Cmd) {
        self.cmd = cmd;
    }
}
