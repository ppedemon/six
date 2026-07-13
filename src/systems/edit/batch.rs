use anyhow::Result;
use ropey::Rope;

use crate::{
    cmd::EditOp,
    components::{Buffer, BufferView, Config, EditorCtx, EditorState, Session},
    systems::{
        commons::{char_idx_to_coords, cursor_to_char_idx},
        edit::{
            buffer::Damage,
            session::{DamageEvent, broadcast_damage},
        },
    },
};

pub fn apply_insert_log(ctx: &EditorCtx, ops: &Vec<EditOp>, reps: usize) -> Result<()> {
    let damage_evt = intepret_insert_log(ctx, ops, reps)?;

    let session_id = {
        let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
        editor.session_id
    };
    broadcast_damage(ctx, session_id, damage_evt)
}

pub fn intepret_insert_log(ctx: &EditorCtx, ops: &Vec<EditOp>, reps: usize) -> Result<DamageEvent> {
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
    let mut q_session = ctx
        .world
        .query_one::<(&mut Session, &mut BufferView)>(editor.session_id);
    let (session, buf_view) = q_session.get()?;

    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut buffer = ctx.world.get::<&mut Buffer>(session.buf_id)?;
    let mut interpreter = InsertLogInterpreter::new(&config, buf_view, &mut buffer.rope);

    let damage = interpreter.interpret(ops, reps);
    Ok(DamageEvent::new(session.buf_id, damage))
}

struct InsertLogInterpreter<'a> {
    config: &'a Config,
    buf_view: &'a mut BufferView,
    rope: &'a mut Rope,
    row: usize,
    char_idx: usize,
    destroy_from: Option<usize>,
}

// NOTE: This intepreter uses a Rope, not a DisplayBuffer. That means it knows
// nothing about multi-char graphemes, but only about single chars. Therefore,
// backspace and delete remove *single characters*. If the insert log happens
// to execute a delete or backspace on a multi-char grapheme, it might not
// remove it completely, and if that happens the result will look weird.
//
// Anyway, I'm leaving it like this for now. Adding DisplayBuffer recalculation
// and Unicode segmentation to the interpreter's execution would be prohibitely
// expensive.
//
// Never forget: multichar graphemes are hell.
impl<'a> InsertLogInterpreter<'a> {
    fn new(config: &'a Config, buf_view: &'a mut BufferView, rope: &'a mut Rope) -> Self {
        let row = buf_view.cursor.row;
        let char_idx = cursor_to_char_idx(config, buf_view, rope);

        Self {
            config,
            buf_view,
            rope,
            row,
            char_idx,
            destroy_from: None,
        }
    }

    fn interpret(&mut self, log: &Vec<EditOp>, reps: usize) -> Damage {
        if reps == 0 {
            return Damage::Intact;
        }

        for _ in 0..reps {
            for op in log {
                match op {
                    EditOp::Esc => {}
                    EditOp::InsertChar(c) => self.insert(*c),
                    EditOp::Tab => self.insert('\t'),
                    EditOp::Enter => self.enter(),
                    EditOp::Backspace => self.backspace(),
                    EditOp::Delete => self.delete(),
                }
            }
        }

        let damage: Damage;
        if let Some(row) = self.destroy_from {
            self.buf_view.display_buf.destroy_from(row);
            damage = Damage::From(row);
        } else {
            self.buf_view
                .display_buf
                .patch_range(self.config, self.rope, self.row..self.row + 1);
            damage = Damage::Line(self.row);
        }

        let coords = char_idx_to_coords(self.config, self.rope, self.buf_view, self.char_idx);
        self.buf_view.cursor = coords;
        damage
    }

    fn insert(&mut self, c: char) {
        if c == '\n' || c == '\r' {
            self.enter()
        } else {
            self.rope.insert_char(self.char_idx, c);
            self.char_idx += 1;
        }
    }

    fn enter(&mut self) {
        self.rope.insert_char(self.char_idx, '\n');
        self.set_rebuild_from(self.row);
        self.row += 1;
        self.char_idx += 1;
    }

    fn backspace(&mut self) {
        if self.char_idx > 0 {
            let mut prev_idx = self.char_idx - 1;
            let c = self.rope.char(prev_idx);
            if c == '\n' {
                self.row -= 1;
                self.set_rebuild_from(self.row);
                if prev_idx > 0 && self.rope.char(prev_idx - 1) == '\r' {
                    prev_idx -= 1;
                }
            }
            self.rope.remove(prev_idx..self.char_idx);
            self.char_idx = prev_idx;
        }
    }

    fn delete(&mut self) {
        let max_doc_idx = self.rope.len_chars().saturating_sub(1);

        if self.char_idx < max_doc_idx {
            let mut next_idx = self.char_idx + 1;
            let c = self.rope.char(next_idx);
            if c == '\n' || c == '\r' {
                self.set_rebuild_from(self.row);
                if c == '\r' && next_idx < max_doc_idx && self.rope.char(next_idx + 1) == '\n' {
                    next_idx += 1;
                }
            }
            self.rope.remove(self.char_idx..next_idx);
        }
    }

    fn set_rebuild_from(&mut self, row: usize) {
        match self.destroy_from {
            None => self.destroy_from = Some(row),
            Some(old_row) => self.destroy_from = Some(old_row.min(row)),
        }
    }
}
