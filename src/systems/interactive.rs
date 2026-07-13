use anyhow::Result;

use crate::{
    cmd::{Cmd, InsertPoint, InteractiveOp},
    components::{BufferView, EditorCtx, EditorState},
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

pub fn handle_interactive(ctx: &EditorCtx, args: InteractiveArgs) -> Result<()> {
    match args.op {
        InteractiveOp::OpenAbove => open_above(ctx, args.cmd),
        InteractiveOp::OpenBelow => open_below(ctx, args.cmd),
    }
}

fn open_above(ctx: &EditorCtx, cmd: Cmd) -> Result<()> {
    {
        let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
        let mut buf_view = ctx.world.get::<&mut BufferView>(editor.session_id)?;
        buf_view.cursor.col = 0;
    }

    enter_insert(ctx, InsertPoint::Curr, cmd)?;
    insert::utils::open_line(ctx)?;
    nav::utils::cursor_up::<InsertNav>(ctx)
}

fn open_below(ctx: &EditorCtx, cmd: Cmd) -> Result<()> {
    enter_insert(ctx, InsertPoint::Last, cmd)?;
    insert::utils::open_line(ctx)
}
