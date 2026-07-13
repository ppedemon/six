use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::{
    cmd::{EditOp, InsertOp, Motion},
    components::{EditorCtx, EditorState},
    digraphs,
    systems::{input::handler::dispatch_insert, status},
};

enum State {
    Immediate,
    Digraph0,
    Digraph1(char),
}

pub struct InsertInputHandler {
    state: State,
    dg_buf: String,
}

impl InsertInputHandler {
    pub fn new() -> Self {
        Self {
            state: State::Immediate,
            dg_buf: String::with_capacity(4),
        }
    }

    pub fn handle_event(&mut self, ctx: &EditorCtx, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) => self.handle_key(ctx, key_event),
            _ => Ok(()),
        }
    }

    fn handle_key(&mut self, ctx: &EditorCtx, event: KeyEvent) -> Result<()> {
        match self.state {
            State::Immediate => self.handle_immediate(ctx, event),
            State::Digraph0 => self.handle_digraph0(ctx, event),
            State::Digraph1(c) => self.handle_digraph1(ctx, c, event),
        }
    }

    fn handle_immediate(&mut self, ctx: &EditorCtx, event: KeyEvent) -> Result<()> {
        let op = match event.code {
            KeyCode::Char('k') | KeyCode::Char('K') if event.modifiers == KeyModifiers::CONTROL => {
                self.dg_start(ctx)?;
                return Ok(());
            }

            KeyCode::Esc => InsertOp::Esc,

            KeyCode::Char(c) => EditOp::InsertChar(c).into(),
            KeyCode::Enter => EditOp::Enter.into(),
            KeyCode::Backspace => EditOp::Backspace.into(),
            KeyCode::Delete => EditOp::Delete.into(),
            KeyCode::Tab => EditOp::Tab.into(),

            KeyCode::Up => Motion::Up.into(),
            KeyCode::Down => Motion::Down.into(),
            KeyCode::Left => Motion::Left.into(),
            KeyCode::Right => Motion::Right.into(),
            KeyCode::PageUp => Motion::PageUp.into(),
            KeyCode::PageDown => Motion::PageDown.into(),

            KeyCode::Home => {
                if event.modifiers == KeyModifiers::CONTROL {
                    Motion::StartOfFile.into()
                } else {
                    Motion::StartOfLine.into()
                }
            }
            KeyCode::End => {
                if event.modifiers == KeyModifiers::CONTROL {
                    Motion::EndOfFile.into()
                } else {
                    Motion::EndOfLine.into()
                }
            }
            _ => return Ok(()),
        };
        dispatch_insert(ctx, op)
    }

    fn handle_digraph0(&mut self, ctx: &EditorCtx, event: KeyEvent) -> Result<()> {
        match event.code {
            KeyCode::Char(c) => self.dg_first(ctx, c),
            _ => self.dg_esc(ctx),
        }
    }

    fn handle_digraph1(&mut self, ctx: &EditorCtx, c0: char, event: KeyEvent) -> Result<()> {
        match event.code {
            KeyCode::Char(c1) => {
                let op = self.dg_end(ctx, c0, c1)?;
                dispatch_insert(ctx, op)
            }
            _ => self.dg_esc(ctx),
        }
    }

    fn dg_start(&mut self, ctx: &EditorCtx) -> Result<()> {
        self.state = State::Digraph0;
        self.dg_buf.clear();
        self.dg_buf.push_str("DG");

        status::set_cmd(ctx, &self.dg_buf)?;
        set_char_at_cursor(ctx, '?')
    }

    fn dg_esc(&mut self, ctx: &EditorCtx) -> Result<()> {
        self.state = State::Immediate;
        self.dg_buf.clear();

        status::clear_cmd(ctx)?;
        clear_char_at_cursor(ctx)
    }

    fn dg_first(&mut self, ctx: &EditorCtx, c: char) -> Result<()> {
        self.state = State::Digraph1(c);
        self.dg_buf.push(':');
        self.dg_buf.push(c);

        status::set_cmd(ctx, &self.dg_buf)?;
        set_char_at_cursor(ctx, c)
    }

    fn dg_end(&mut self, ctx: &EditorCtx, c0: char, c1: char) -> Result<InsertOp> {
        self.state = State::Immediate;
        self.dg_buf.clear();

        status::clear_cmd(ctx)?;
        clear_char_at_cursor(ctx)?;

        let final_char = if let Some(c) = digraphs::get(c0, c1) {
            c
        } else {
            c1
        };

        Ok(EditOp::InsertChar(final_char).into())
    }
}

pub fn set_char_at_cursor(ctx: &EditorCtx, c: char) -> Result<()> {
    let mut editor = ctx.world.get::<&mut EditorState>(ctx.editor_id)?;
    editor.char_at_cursor = Some(c);
    Ok(())
}

pub fn clear_char_at_cursor(ctx: &EditorCtx) -> Result<()> {
    let mut editor = ctx.world.get::<&mut EditorState>(ctx.editor_id)?;
    editor.char_at_cursor = None;
    Ok(())
}
