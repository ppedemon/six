use super::log;
use crate::{
    active_session,
    cmd::{Cmd, EditOp, InteractiveOp, Operator, SysOp},
    components::EditorCtx,
};

pub fn post_insert(ctx: &mut EditorCtx) {
    let (ops, repetable) = get_insert_log(ctx);
    commit_to_regs(ctx, ops);
    if repetable {
        post_insert_repeat(ctx);
    }
}

fn get_insert_log(ctx: &mut EditorCtx) -> (Vec<EditOp>, bool) {
    let (session, _) = active_session!(mut ctx);
    session.insert_log.take_log()
}

fn commit_to_regs(ctx: &mut EditorCtx, insert_log: Vec<EditOp>) {
    // Open commands add a leading Enter that we don't want to store in the ". register
    // let ops = if is_open_cmd(ctx.repbuf.last_cmd()) {
    //     &insert_log[1..]
    // } else {
    //     &insert_log
    // };
    ctx.registers.commit_insert_log(insert_log);
}

fn is_open_cmd(cmd: Option<Cmd>) -> bool {
    cmd.is_some_and(|cmd| {
        cmd.op == Operator::Interactive(InteractiveOp::OpenAbove)
            || cmd.op == Operator::Interactive(InteractiveOp::OpenBelow)
    })
}

fn post_insert_repeat(ctx: &mut EditorCtx) {
    if let Some(cmd) = ctx.repbuf.last_cmd()
        && is_interactive(cmd)
    {
        let ops = &ctx.registers.last_insert().to_vec();
        let reps = cmd.reps.unwrap_or(1).saturating_sub(1);
        apply_insert_log(ctx, cmd, ops, reps);
    }
}

fn is_interactive(cmd: Cmd) -> bool {
    match cmd.op {
        Operator::Sys(SysOp::EnterInsert(_)) | Operator::Interactive(_) => true,
        _ => false,
    }
}

fn apply_insert_log(ctx: &mut EditorCtx, cmd: Cmd, ops: &[EditOp], reps: usize) {
    match cmd.op {
        Operator::Sys(SysOp::EnterInsert(_)) => log::apply_insert_log(ctx, ops, reps),
        Operator::Interactive(InteractiveOp::OpenAbove)
        | Operator::Interactive(InteractiveOp::OpenBelow) => {
            let mut new_ops = Vec::with_capacity(ops.len() + 1);
            new_ops.push(EditOp::Enter);
            new_ops.extend_from_slice(ops);
            log::apply_insert_log(ctx, &new_ops, reps);
        }
        _ => {}
    }
}
