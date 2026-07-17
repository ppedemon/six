use crate::{
    cmd::{Cmd, ImmediateOp},
    components::EditorCtx,
    systems::{
        commons::{coords_to_char_idx, curr_line},
        insert::{Damage, DamageEvent, broadcast_damage},
        nav::utils::ensure_cursor_inside_line,
    },
};

pub struct ImmediateArgs {
    op: ImmediateOp,
    cmd: Cmd,
}

impl ImmediateArgs {
    pub fn new(op: ImmediateOp, cmd: Cmd) -> Self {
        Self { op, cmd }
    }
}

pub fn handle_immediate(ctx: &mut EditorCtx, args: ImmediateArgs) {
    let damage = match args.op {
        ImmediateOp::DeleteChar => {
            let damage = delete_char(ctx, args.cmd.reps.unwrap_or(1));
            ensure_cursor_inside_line(ctx);
            damage
        }
    };

    let (session, _) = ctx.active_session();
    let damage_evt = DamageEvent::new(session.buf_id, damage);
    broadcast_damage(ctx, damage_evt);
}

// TODO Register bookkeeping
fn delete_char(ctx: &mut EditorCtx, reps: usize) -> Damage {
    let (session, buf_view) = ctx.sessions.get_mut(&ctx.editor.session_id).unwrap();
    let buffer = ctx.buffers.get_mut(&session.buf_id).unwrap();

    let start_coords = buf_view.cursor;
    let mut end_coords = start_coords;
    let line = curr_line(&ctx.config, &buffer.rope, buf_view);

    if line.display_width == 0 {
        return Damage::Intact;
    }

    for _ in 0..reps {
        let prev_col = end_coords.col;
        end_coords.col = line.next_col(end_coords.col);
        if prev_col == end_coords.col {
            end_coords.col = line.display_width;
            break;
        }
    }

    let start_idx = coords_to_char_idx(&ctx.config, &buffer.rope, buf_view, start_coords);
    let end_idx = coords_to_char_idx(&ctx.config, &buffer.rope, buf_view, end_coords);
    buffer.rope.remove(start_idx..end_idx);
    buffer.dirty = true;

    buf_view.display_buf.patch_range(
        &ctx.config,
        &buffer.rope,
        start_coords.row..start_coords.row + 1,
    );

    Damage::Line(buf_view.cursor.row)
}
