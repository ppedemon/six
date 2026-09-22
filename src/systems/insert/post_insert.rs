use crate::{
    active_session, active_session_and_buffer,
    cmd::{EditOp, Operator},
    components::EditorCtx,
    systems::{
        commons, interactive,
        nav::{NormalNav, move_left},
    },
};

pub fn post_insert(ctx: &mut EditorCtx) {
    let (ops, repetable) = get_insert_log(ctx);
    commit_to_regs(ctx, ops);
    if repetable {
        post_insert_repeat(ctx);
    }
    restore_cursor(ctx);
}

fn get_insert_log(ctx: &mut EditorCtx) -> (Vec<EditOp>, bool) {
    let (session, _) = active_session!(mut ctx);
    session.insert_log.take_log()
}

fn commit_to_regs(ctx: &mut EditorCtx, insert_log: Vec<EditOp>) {
    ctx.registers.commit_insert_log(insert_log);
}

fn post_insert_repeat(ctx: &mut EditorCtx) {
    if let Some(cmd) = ctx.repbuf.last_cmd() {
        if let Operator::Interactive(op) = cmd.op {
            let reps = cmd.reps.unwrap_or(1).saturating_sub(1);
            interactive::finish_interactive(ctx, op, reps);
        }
    }
}

fn restore_cursor(ctx: &mut EditorCtx) {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let cursor = buf_view.cursor;
    let line = commons::curr_line(&ctx.config, buffer.rope(), buf_view);
    buf_view.cursor.col = line.snap_col(cursor.col);
    move_left::<NormalNav>(&ctx.config, buffer.rope(), buf_view, 1);
}
