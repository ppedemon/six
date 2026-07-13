use anyhow::Result;
use crossterm::event::Event;

use crate::{
    cmd::{Cmd, InsertOp, Operator}, components::{EditorCtx, EditorState, Focus, Mode, Session}, systems::{
        input::{insert::InsertInputHandler, normal::NormalInputHandler}, insert::handle_edit, interactive::{InteractiveArgs, handle_interactive}, nav::{NavArgs, handle_nav}, search::{SearchArgs, handle_search}, sys::{SysArgs, enter_normal, handle_sys},
    },
};

pub struct InputHandler {
    normal: NormalInputHandler,
    insert: InsertInputHandler,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            normal: NormalInputHandler::new(),
            insert: InsertInputHandler::new(),
        }
    }

    pub fn handle_event(&mut self, ctx: &EditorCtx, event: Event) -> Result<()> {
        let (focus, session_id) = {
            let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
            (editor.focus, editor.session_id)
        };
        match focus {
            Focus::Ex => self.insert.handle_event(ctx, event),
            Focus::Session => {
                let mode = {
                    let session = ctx.world.get::<&Session>(session_id)?;
                    session.mode
                };
                match mode {
                    Mode::Insert => self.insert.handle_event(ctx, event),
                    Mode::Normal => self.normal.handle_event(ctx, event),
                }
            }
        }
    }
}

pub fn dispatch_cmd(ctx: &EditorCtx, cmd: Cmd) -> Result<()> {
    let reps = cmd.reps;
    match cmd.op {
        Operator::Nop => Ok(()),
        Operator::Sys(op) => {
            let args = SysArgs::new(op, cmd);
            handle_sys(ctx, args)
        }
        Operator::Move(motion) => {
            let args = NavArgs::new(motion, cmd);
            handle_nav(ctx, args)
        }
        Operator::Search(op) => {
            let args = SearchArgs::new(op, cmd);
            handle_search(ctx, args)
        }
        Operator::Interactive(op) => {
            let args = InteractiveArgs::new(op, cmd);
            handle_interactive(ctx, args)
        }
    }
}

pub fn dispatch_insert(ctx: &EditorCtx, op: InsertOp) -> Result<()> {
    match op {
        InsertOp::Esc => enter_normal(ctx),
        InsertOp::Edit(edit_op) => handle_edit(ctx, edit_op),
        InsertOp::Move(motion) => {
            let nav_args = NavArgs::new(motion, Cmd::new(motion.into()));
            handle_nav(ctx, nav_args)
        }
    }
}
