use crate::{
    active_session,
    cmd::{EditOp, Operator},
    components::EditorCtx,
    systems::interactive,
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
    ctx.registers.commit_insert_log(insert_log);
}

fn post_insert_repeat(ctx: &mut EditorCtx) {
    if let Some(cmd) = ctx.repbuf.last_cmd() {
        if let Operator::Interactive(op) = cmd.op {
            let reps = cmd.reps.unwrap_or(1).saturating_sub(1);
            interactive::exec_prologue(ctx, op, reps);
        }
    }
}
