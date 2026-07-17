use crate::{
    cmd::{Cmd, InsertPoint, InteractiveOp},
    components::EditorCtx,
    systems::{
        enter_insert, insert,
        nav::{self, InsertNav},
    },
};

pub struct InteractiveArgs {
    op: InteractiveOp,
    cmd: Cmd,
}

impl InteractiveArgs {
    pub fn new(op: InteractiveOp, cmd: Cmd) -> Self {
        Self { op, cmd }
    }
}

pub fn handle_interactive(ctx: &mut EditorCtx, args: InteractiveArgs) {
    match args.op {
        InteractiveOp::OpenAbove => open_above(ctx, args.cmd),
        InteractiveOp::OpenBelow => open_below(ctx, args.cmd),
    }
}

fn open_above(ctx: &mut EditorCtx, cmd: Cmd) {
    let (session, buf_view) = ctx.sessions.get_mut(&ctx.editor.session_id).unwrap();
    let buffer = ctx.buffers.get_mut(&session.buf_id).unwrap();

    buf_view.cursor.col = 0;
    enter_insert(ctx, InsertPoint::Curr, cmd);
    insert::utils::open_line(ctx);
    nav::utils::cursor_up::<InsertNav>(ctx)
}

fn open_below(ctx: &mut EditorCtx, cmd: Cmd) {
    enter_insert(ctx, InsertPoint::Last, cmd);
    insert::utils::open_line(ctx)
}
