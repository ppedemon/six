use ratatui::layout::{Position, Rect};
use ropey::Rope;

use crate::components::{DisplayBuffer, InsertLog};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coords {
    pub row: usize,
    pub col: usize,
}

impl Coords {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

impl Default for Coords {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub scroll: Coords,
    pub area: Rect,
}

impl Viewport {
    pub fn cursor_pos(&self, cursor: Coords) -> Position {
        let x = cursor.col - self.scroll.col;
        let y = cursor.row - self.scroll.row;
        Position::new(self.area.x + x as u16, self.area.y + y as u16)
    }

    pub fn pg_size(&self, page_scroll_margin: u16) -> usize {
        self.area.height.saturating_sub(page_scroll_margin).max(2) as usize
    }

    pub fn scroll_to_row(&mut self, row: usize) {
        self.scroll.row = row;
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            scroll: Coords::default(),
            area: Rect::default(),
        }
    }
}

pub struct BufferView {
    pub cursor: Coords,
    pub target_col: usize,
    pub display_buf: DisplayBuffer,
}

impl BufferView {
    pub fn empty() -> Self {
        Self {
            cursor: Coords::default(),
            target_col: 0,
            display_buf: DisplayBuffer::empty(),
        }
    }
}

pub struct Session {
    pub mode: Mode,
    pub buf_id: hecs::Entity,
    pub viewport: Viewport,
    pub insert_log: InsertLog,
}

impl Session {
    pub fn new(buf_id: hecs::Entity) -> Self {
        Self {
            mode: Mode::Normal,
            buf_id,
            viewport: Viewport::default(),
            insert_log: InsertLog::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExState {
    Idle,
    Cancel,
    Submit(String),
}

pub struct ExSession {
    pub viewport: Viewport,
    pub rope: Rope,
    pub state: ExState,
}

impl ExSession {
    pub fn new() -> Self {
        Self {
            viewport: Viewport::default(),
            rope: Rope::new(),
            state: ExState::Idle,
        }
    }
}
