use anyhow::Result;
use crate::components::{EditorCtx, Level, Status};

pub fn clear_msg(ctx: &EditorCtx) -> Result<()> {
  let mut status = ctx.world.get::<&mut Status>(ctx.status_id)?;
  status.clear_msg();
  Ok(())
}

pub fn clear_cmd(ctx: &EditorCtx) -> Result<()> {
  let mut status = ctx.world.get::<&mut Status>(ctx.status_id)?;
  status.clear_cmd();
  Ok(())
}

pub fn clear_ruler(ctx: &EditorCtx) -> Result<()> {
  let mut status = ctx.world.get::<&mut Status>(ctx.status_id)?;
  status.clear_ruler();
  Ok(())
}

pub fn set_msg(ctx: &EditorCtx, level: Level, msg: &str) -> Result<()> {
  let mut status = ctx.world.get::<&mut Status>(ctx.status_id)?;
  status.set_msg(level, msg);
  Ok(())
}

pub fn set_cmd(ctx: &EditorCtx, cmd_msg: &str) -> Result<()> {
  let mut status = ctx.world.get::<&mut Status>(ctx.status_id)?;
  status.set_cmd(cmd_msg);
  Ok(())
}

pub fn set_ruler(ctx: &EditorCtx, ruler_msg: &str) -> Result<()> {
  let mut status = ctx.world.get::<&mut Status>(ctx.status_id)?;
  status.set_ruler(ruler_msg);
  Ok(())
}
