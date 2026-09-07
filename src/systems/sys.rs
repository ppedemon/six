use crate::{
    active_session_and_buffer,
    cmd::{Cmd, SysOp},
    components::EditorCtx,
    ex::ExRange,
    systems::{
        commons::cursor_to_char_idx,
        ex,
        mode::{enter_ex, enter_normal},
        quit_editor,
    },
};

pub struct SysArgs {
    pub op: SysOp,
    cmd: Cmd,
}

impl SysArgs {
    pub fn new(op: SysOp, cmd: Cmd) -> Self {
        Self { op, cmd }
    }
}

pub fn handle_sys(ctx: &mut EditorCtx, args: SysArgs) {
    match args.op {
        SysOp::EnterNormal => enter_normal(ctx),
        SysOp::EnterEx(ex_mode) => enter_ex(ctx, ex_mode),
        SysOp::HardQuit => quit_editor(ctx),
        SysOp::CondWriteAndQuit => {
            ex::save_active(ctx, None, false, true, ExRange::All).unwrap();
            quit_editor(ctx)
        }
        SysOp::AddLocalMark(c) => add_local_mark(ctx, c),
    }
}

fn add_local_mark(ctx: &mut EditorCtx, c: char) {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let char_idx = cursor_to_char_idx(&ctx.config, buf_view, buffer.rope());
    buffer.marks_mut().write(c, char_idx);
}
