use crate::{
    cmd::{Cmd, ImmediateOp},
    components::{Buffer, Config, EditorCtx, Session},
    systems::{
        commons::{active_session_id, coords_to_char_idx, curr_line, mut_active_session_query},
        insert::{Damage, DamageEvent, broadcast_damage},
        nav::utils::ensure_cursor_inside_line,
    },
};
use anyhow::Result;

pub struct ImmediateArgs {
    op: ImmediateOp,
    cmd: Cmd,
}

impl ImmediateArgs {
    pub fn new(op: ImmediateOp, cmd: Cmd) -> Self {
        Self { op, cmd }
    }
}

pub fn handle_immediate(ctx: &EditorCtx, args: ImmediateArgs) -> Result<()> {
    let damage = match args.op {
        ImmediateOp::DeleteChar => {
            let damage = delete_char(ctx, args.cmd.reps.unwrap_or(1))?;
            ensure_cursor_inside_line(ctx)?;
            damage
        }
    };

    let session_id = active_session_id(ctx)?;
    let session = ctx.world.get::<&Session>(session_id)?;
    broadcast_damage(ctx, session_id, DamageEvent::new(session.buf_id, damage))
}

// TODO Register bookkeeping
fn delete_char(ctx: &EditorCtx, reps: usize) -> Result<Damage> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_session = mut_active_session_query(ctx)?;
    let (session, buf_view) = q_session.get()?;
    let mut buffer = ctx.world.get::<&mut Buffer>(session.buf_id)?;

    let start_coords = buf_view.cursor;
    let mut end_coords = start_coords;
    let line = curr_line(&config, &buffer.rope, buf_view);

    if line.display_width == 0 {
        return Ok(Damage::Intact);
    }

    for _ in 0..reps {
        let prev_col = end_coords.col;
        end_coords.col = line.next_col(end_coords.col);
        if prev_col == end_coords.col {
            end_coords.col = line.display_width;
            break;
        }
    }

    let start_idx = coords_to_char_idx(&config, &buffer.rope, buf_view, start_coords);
    let end_idx = coords_to_char_idx(&config, &buffer.rope, buf_view, end_coords);
    buffer.rope.remove(start_idx..end_idx);
    buffer.dirty = true;

    buf_view.display_buf.patch_range(
        &config,
        &buffer.rope,
        start_coords.row..start_coords.row + 1,
    );

    Ok(Damage::Line(buf_view.cursor.row))
}
