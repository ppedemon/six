use crossterm::event::Event;

use crate::{
    active_session,
    cmd::{Cmd, InsertOp, Operator},
    components::{EditorCtx, Focus, Mode},
    systems::{
        immediate::{ImmediateArgs, handle_immediate},
        input::{insert::InsertInputHandler, normal::NormalInputHandler},
        insert::handle_edit,
        interactive::{InteractiveArgs, handle_interactive},
        nav::{NavArgs, handle_nav},
        sys::{SysArgs, enter_normal, handle_sys},
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

    pub fn handle_event(&mut self, ctx: &mut EditorCtx, event: Event) {
        match ctx.editor.focus {
            Focus::Ex => self.insert.handle_event(ctx, event),
            Focus::Session => {
                let mode = {
                    let (session, _) = active_session!(ctx);
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

pub fn dispatch_cmd(ctx: &mut EditorCtx, cmd: Cmd) {
    let reps = cmd.reps;
    match cmd.op {
        Operator::Nop => {}
        Operator::Sys(op) => {
            let args = SysArgs::new(op, cmd);
            handle_sys(ctx, args)
        }
        Operator::Move(motion) => {
            let args = NavArgs::new(motion, cmd);
            handle_nav(ctx, args)
        }
        Operator::Interactive(op) => {
            let args = InteractiveArgs::new(op, cmd);
            handle_interactive(ctx, args)
        }
        Operator::Immediate(op) => {
            let args = ImmediateArgs::new(op, cmd);
            handle_immediate(ctx, args)
        }
    }
}

pub fn dispatch_insert(ctx: &mut EditorCtx, op: InsertOp) {
    match op {
        InsertOp::Esc => enter_normal(ctx),
        InsertOp::Edit(edit_op) => handle_edit(ctx, edit_op),
        InsertOp::Move(motion) => {
            let nav_args = NavArgs::new(motion, Cmd::new(motion.into()));
            handle_nav(ctx, nav_args)
        }
    }
}
