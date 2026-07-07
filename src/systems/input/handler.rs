use anyhow::Result;
use crossterm::event::Event;

use crate::{
    cmd::{Cmd, Operator},
    components::{EditorCtx, EditorState, Focus, Mode, Session},
    systems::{
        edit::handle_edit,
        input::{insert::InsertInputHandler, normal::NormalInputHandler},
        nav::{NavArgs, handle_nav},
        sys::{SysArgs, handle_sys},
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

pub fn dispatch(ctx: &EditorCtx, cmd: Cmd) -> Result<()> {
    let reps = cmd.reps;
    match cmd.op {
        Operator::Nop => Ok(()),
        Operator::Sys(op) => {
            let sys_args = SysArgs::new(op, reps, cmd.target);
            handle_sys(ctx, sys_args)
        }
        Operator::Edit(op) => handle_edit(ctx, op),
        Operator::Move(motion) => {
            let nav_args = NavArgs::new(motion, reps);
            handle_nav(ctx, nav_args)
        }
    }
}
