use super::log;
use crate::{
    active_session,
    cmd::{Cmd, EditOp, InteractiveOp, Operator, SysOp},
    components::{EditorCtx, RepeatBufferItem},
};

pub fn post_insert(ctx: &mut EditorCtx) {
    let ops = get_insert_log(ctx);
    commit_to_regs(ctx, ops.clone());
    commit_to_repbuf(ctx, ops);
    post_insert_repeat(ctx);
}

fn get_insert_log(ctx: &mut EditorCtx) -> Vec<EditOp> {
    let (session, _) = active_session!(mut ctx);
    session.insert_log.take_log()
}

fn commit_to_regs(ctx: &mut EditorCtx, insert_log: Vec<EditOp>) {
    ctx.registers.commit_insert_log(insert_log);
}

fn commit_to_repbuf(ctx: &mut EditorCtx, insert_log: Vec<EditOp>) {
    ctx.repbuf.finish_interaction(insert_log);
}

fn post_insert_repeat(ctx: &mut EditorCtx) {
    let (cmd, ops) = match ctx.repbuf.item() {
        RepeatBufferItem::Interactive(cmd, ops) => (*cmd, ops.clone()),
        _ => return,
    };

    let reps = cmd.reps.unwrap_or(1).saturating_sub(1);
    apply_insert_log(ctx, &cmd, &ops, reps);
}

fn apply_insert_log(ctx: &mut EditorCtx, cmd: &Cmd, ops: &Vec<EditOp>, reps: usize) {
    match cmd.op {
        Operator::Sys(SysOp::EnterInsert(_)) => log::apply_insert_log(ctx, ops, reps),
        Operator::Interactive(InteractiveOp::OpenAbove)
        | Operator::Interactive(InteractiveOp::OpenBelow) => {
            let mut new_ops = Vec::with_capacity(ops.len() + 1);
            new_ops.push(EditOp::Enter);
            new_ops.extend_from_slice(ops);
            log::apply_insert_log(ctx, ops, reps);
        }
        _ => {}
    }
}
