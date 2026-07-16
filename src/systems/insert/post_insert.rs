use anyhow::Result;

use super::log;
use crate::{
    cmd::{Cmd, EditOp, InteractiveOp, Operator, SysOp},
    components::{EditorCtx, Registers, RepeatBuffer, RepeatBufferItem, Session},
    systems::commons::active_session_id,
};

pub fn post_insert(ctx: &EditorCtx) -> Result<()> {
    let ops = get_insert_log(ctx)?;
    commit_to_regs(ctx, ops.clone())?;
    commit_to_repbuf(ctx, ops)?;
    post_insert_repeat(ctx)
}

fn get_insert_log(ctx: &EditorCtx) -> Result<Vec<EditOp>> {
    let session_id = active_session_id(ctx)?;
    let mut session = ctx.world.get::<&mut Session>(session_id)?;
    let log = std::mem::take(&mut session.insert_log.log);
    Ok(log)
}

fn commit_to_regs(ctx: &EditorCtx, insert_log: Vec<EditOp>) -> Result<()> {
    let mut registers = ctx.world.get::<&mut Registers>(ctx.registers_id)?;
    registers.commit_insert_log(insert_log);
    Ok(())
}

fn commit_to_repbuf(ctx: &EditorCtx, insert_log: Vec<EditOp>) -> Result<()> {
    let mut repbuf = ctx.world.get::<&mut RepeatBuffer>(ctx.repbuf_id)?;
    repbuf.finish_interaction(insert_log);
    Ok(())
}

fn post_insert_repeat(ctx: &EditorCtx) -> Result<()> {
    let repbuf = ctx.world.get::<&RepeatBuffer>(ctx.repbuf_id)?;
    match &repbuf.item {
        RepeatBufferItem::Interactive(cmd, ops) => {
            let reps = cmd.reps.unwrap_or(1).saturating_sub(1);
            apply_insert_log(ctx, cmd, ops, reps)?;
        }
        _ => {}
    }
    Ok(())
}

fn apply_insert_log(ctx: &EditorCtx, cmd: &Cmd, ops: &Vec<EditOp>, reps: usize) -> Result<()> {
    match cmd.op {
        Operator::Sys(SysOp::EnterInsert(_)) => log::apply_insert_log(ctx, ops, reps)?,
        Operator::Interactive(InteractiveOp::OpenAbove)
        | Operator::Interactive(InteractiveOp::OpenBelow) => {
            let mut new_ops = Vec::with_capacity(ops.len() + 1);
            new_ops.push(EditOp::Enter);
            new_ops.extend_from_slice(ops);
            log::apply_insert_log(ctx, ops, reps)?;
        }
        _ => {}
    }
    Ok(())
}
