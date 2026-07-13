use crate::{
    cmd::{Cmd, EditOp, Operator, SysOp},
    components::{CmdItem, EditorCtx, EditorState, Registers, RepeatBuffer, Session},
    systems::edit::batch::apply_insert_log,
};
use anyhow::Result;

pub fn post_edit(ctx: &EditorCtx) -> Result<()> {
    let ops = get_insert_ops(ctx)?;
    commit_to_regs(ctx, ops.clone())?;
    commit_to_repbuf(ctx, ops)?;
    post_edit_repeat(ctx)
}

fn get_insert_ops(ctx: &EditorCtx) -> Result<Vec<EditOp>> {
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
    let mut session = ctx.world.get::<&mut Session>(editor.session_id)?;
    let ops = std::mem::take(&mut session.insert_log.log);
    Ok(ops)
}

fn commit_to_regs(ctx: &EditorCtx, ops: Vec<EditOp>) -> Result<()> {
    let mut registers = ctx.world.get::<&mut Registers>(ctx.registers_id)?;
    registers.commit_insert_log(ops);
    Ok(())
}

fn commit_to_repbuf(ctx: &EditorCtx, ops: Vec<EditOp>) -> Result<()> {
    let mut repbuf = ctx.world.get::<&mut RepeatBuffer>(ctx.repbuf_id)?;
    repbuf.finish_interaction(ops);
    Ok(())
}

fn post_edit_repeat(ctx: &EditorCtx) -> Result<()> {
    let repbuf = ctx.world.get::<&RepeatBuffer>(ctx.repbuf_id)?;
    match &repbuf.cmd_item {
        CmdItem::FullInteraction(cmd, ops) => {
            let reps = cmd.reps.unwrap_or(1).saturating_sub(1);
            apply_interactive_ops(ctx, cmd, ops, reps)?;
        }
        _ => {}
    }
    Ok(())
}

fn apply_interactive_ops(ctx: &EditorCtx, cmd: &Cmd, ops: &Vec<EditOp>, reps: usize) -> Result<()> {
    match cmd.op {
        Operator::Sys(SysOp::EnterInsert(_)) => apply_insert_log(ctx, ops, reps)?,
        Operator::Sys(SysOp::OpenAbove) | Operator::Sys(SysOp::OpenBelow) => {
            let mut new_ops = Vec::with_capacity(ops.len() + 1);
            new_ops.push(EditOp::Enter);
            new_ops.extend_from_slice(ops);
            apply_insert_log(ctx, ops, reps)?;
        }
        _ => {}
    }
    Ok(())
}
