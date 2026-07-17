use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::{
    cmd::{EditOp, InsertOp, Motion},
    components::EditorCtx,
    digraphs,
    systems::input::handler::dispatch_insert,
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

    pub fn handle_event(&mut self, ctx: &mut EditorCtx, event: Event) {
        match event {
            Event::Key(key_event) => self.handle_key(ctx, key_event),
            _ => {}
        }
    }

    fn handle_key(&mut self, ctx: &mut EditorCtx, event: KeyEvent) {
        match self.state {
            State::Immediate => self.handle_immediate(ctx, event),
            State::Digraph0 => self.handle_digraph0(ctx, event),
            State::Digraph1(c) => self.handle_digraph1(ctx, c, event),
        }
    }

    fn handle_immediate(&mut self, ctx: &mut EditorCtx, event: KeyEvent) {
        let op = match event.code {
            KeyCode::Char('k') | KeyCode::Char('K') if event.modifiers == KeyModifiers::CONTROL => {
                self.dg_start(ctx);
                return;
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
            _ => return,
        };
        dispatch_insert(ctx, op)
    }

    fn handle_digraph0(&mut self, ctx: &mut EditorCtx, event: KeyEvent) {
        match event.code {
            KeyCode::Char(c) => self.dg_first(ctx, c),
            _ => self.dg_esc(ctx),
        }
    }

    fn handle_digraph1(&mut self, ctx: &mut EditorCtx, c0: char, event: KeyEvent) {
        match event.code {
            KeyCode::Char(c1) => {
                let op = self.dg_end(ctx, c0, c1);
                dispatch_insert(ctx, op);
            }
            _ => self.dg_esc(ctx),
        }
    }

    fn dg_start(&mut self, ctx: &mut EditorCtx) {
        self.state = State::Digraph0;
        self.dg_buf.clear();
        self.dg_buf.push_str("DG");

        ctx.status.set_cmd(&self.dg_buf);
        set_char_at_cursor(ctx, '?')
    }

    fn dg_esc(&mut self, ctx: &mut EditorCtx) {
        self.state = State::Immediate;
        self.dg_buf.clear();

        ctx.status.clear_cmd();
        clear_char_at_cursor(ctx)
    }

    fn dg_first(&mut self, ctx: &mut EditorCtx, c: char) {
        self.state = State::Digraph1(c);
        self.dg_buf.push(':');
        self.dg_buf.push(c);

        ctx.status.set_cmd(&self.dg_buf);
        set_char_at_cursor(ctx, c)
    }

    fn dg_end(&mut self, ctx: &mut EditorCtx, c0: char, c1: char) -> InsertOp {
        self.state = State::Immediate;
        self.dg_buf.clear();

        ctx.status.clear_cmd();
        clear_char_at_cursor(ctx);

        let final_char = if let Some(c) = digraphs::get(c0, c1) {
            c
        } else {
            c1
        };

        EditOp::InsertChar(final_char).into()
    }
}

pub fn set_char_at_cursor(ctx: &mut EditorCtx, c: char) {
    ctx.editor.char_at_cursor = Some(c);
}

pub fn clear_char_at_cursor(ctx: &mut EditorCtx) {
    ctx.editor.char_at_cursor = None;
}
