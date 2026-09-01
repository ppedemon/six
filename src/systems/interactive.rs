use crate::{
    cmd::{Cmd, EditOp, InsertPoint, InteractiveOp, Motion, Operator::Move},
    components::EditorCtx,
    systems::{
        enter_insert, insert,
        nav::{self, NavArgs},
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
    //let (_, buf_view) = active_session!(mut ctx);

    let tmp_cmd = Cmd::new(Move(Motion::StartOfLine));
    let nav_args = NavArgs::new(Motion::StartOfLine, tmp_cmd);
    nav::handle_session_nav(ctx, nav_args);

    insert::handle_edit(ctx, EditOp::Enter);

    let tmp_cmd = Cmd::new(Move(Motion::Up));
    let nav_args = NavArgs::new(Motion::Up, tmp_cmd);
    nav::handle_session_nav(ctx, nav_args);

    enter_insert(ctx, InsertPoint::Curr, cmd);

    //buf_view.cursor.col = 0;
    // enter_insert(ctx, InsertPoint::Curr, cmd);
    //insert::utils::open_line(ctx);
    // nav::utils::cursor_up::<InsertNav>(ctx)
}

fn open_below(ctx: &mut EditorCtx, cmd: Cmd) {
    let tmp_cmd = Cmd::new(Move(Motion::EndOfLine));
    let nav_args = NavArgs::new(Motion::EndOfLinePlusOne, tmp_cmd);
    nav::handle_session_nav(ctx, nav_args);

    insert::handle_edit(ctx, EditOp::Enter);

    enter_insert(ctx, InsertPoint::Curr, cmd);

    // enter_insert(ctx, InsertPoint::Last, cmd);
    // insert::utils::open_line(ctx)
}
