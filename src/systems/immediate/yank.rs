use crate::{
    active_session,
    cmd::{Arg, Cmd, Motion, MotionMode},
    components::{EditorCtx, RegisterData},
    systems::{
        event,
        nav::{
            charwise, exec_motion, inclusive, select_blockwise, select_charwise, select_linewise,
        },
    },
};

pub fn yank(ctx: &mut EditorCtx, cmd: Cmd) {
    match cmd.arg {
        Arg::Motion { reps, mode, motion } => {
            let cmd_reps = cmd.reps.unwrap_or(1);
            let arg_reps = reps.unwrap_or(1);
            match motion_yank(ctx, motion, cmd_reps, arg_reps, mode) {
                None => {}
                Some(reg_data) => {
                    event::on_yank(&mut ctx.status, &reg_data);
                    ctx.registers.record_yank(cmd.reg, reg_data);
                }
            }
        }
        // TODO Implement text-object movement and selection
        Arg::TextObject { .. } => {}
        Arg::None => {}
    };
}

pub fn motion_yank(
    ctx: &mut EditorCtx,
    m: Motion,
    cmd_reps: usize,
    args_reps: usize,
    forced_mode: Option<MotionMode>,
) -> Option<RegisterData> {
    let (orig_cursor, orig_target_col) = {
        let (_, buf_view) = active_session!(ctx);
        let orig_cursor = buf_view.cursor;
        let orig_target_col = buf_view.target_col;
        (orig_cursor, orig_target_col)
    };

    let extent = exec_motion(ctx, m, cmd_reps, args_reps)?;

    let orig_mode = if charwise(m) {
        MotionMode::Charwise
    } else {
        MotionMode::Linewise
    };

    let mut inclusive = inclusive(ctx, m) || (orig_mode == MotionMode::Charwise && extent.overshot);
    if forced_mode.is_some_and(|mode| mode == MotionMode::Charwise) {
        inclusive = !inclusive;
    }

    let span = (extent.start, extent.end);
    let reg_data = match forced_mode.unwrap_or(orig_mode) {
        MotionMode::Charwise => select_charwise(ctx, span, inclusive),
        MotionMode::Linewise => select_linewise(ctx, span),
        MotionMode::Blockwise => select_blockwise(ctx, span),
    };

    if extent.start < extent.end {
        let (_, buf_view) = active_session!(mut ctx);
        buf_view.cursor = orig_cursor;
        buf_view.target_col = orig_target_col;
    }

    Some(reg_data)
}
