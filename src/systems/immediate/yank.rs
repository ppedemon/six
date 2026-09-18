use crate::{
    active_session, active_session_and_buffer,
    cmd::{Arg, Cmd, Motion, MotionMode},
    components::{Coords, EditorCtx, RegisterData, YankData, YankShape},
    systems::{
        commons::{char_idx_to_coords, coords_to_char_idx},
        event,
        nav::{
            MotionExtent, charwise, exec_motion, inclusive, select_blockwise, select_charwise,
            select_charwise_nl, select_linewise,
        },
    },
};

// This is the implementation is the yank command (y)
pub fn yank(ctx: &mut EditorCtx, cmd: Cmd) {
    match cmd.arg {
        Arg::Motion { reps, mode, motion } => {
            let cmd_reps = cmd.reps.unwrap_or(1);
            let arg_reps = reps.unwrap_or(1);
            match motion_yank(ctx, motion, cmd_reps, arg_reps, mode) {
                None => {}
                Some((reg_data, yank_data)) => {
                    event::on_yank(&mut ctx.status, &reg_data);
                    ctx.registers.record_yank(cmd.reg, reg_data);
                    ctx.repbuf.save_last_yank(yank_data);
                }
            }
        }
        // TODO Implement text-object movement and selection
        Arg::TextObject { .. } => {}
        Arg::None => {}
    };
}

// Do a yank for the given motion and reps
pub fn motion_yank(
    ctx: &mut EditorCtx,
    m: Motion,
    cmd_reps: usize,
    args_reps: usize,
    forced_mode: Option<MotionMode>,
) -> Option<(RegisterData, YankData)> {
    let (register_data, _, yank_data) = gen_motion_yank(ctx, m, cmd_reps, args_reps, forced_mode)?;
    Some((register_data, yank_data))
}

// Do a yank for the given motion and reps, but adapted for the 'c' command.
// We will need to adapt if the extent was covered by the 'w' or 'W' commands,
// since those will, in general, leave us at the beginning of the next word.
// See adjust_for_c_cmd for details about the adapt procedure.
pub fn motion_yank_for_c_cmd(
    ctx: &mut EditorCtx,
    m: Motion,
    cmd_reps: usize,
    args_reps: usize,
    forced_mode: Option<MotionMode>,
) -> Option<(RegisterData, YankData)> {
    let cursor = {
        let (_, buf_view) = active_session!(ctx);
        buf_view.cursor
    };

    let (mut register_data, mut extent, yank_data) =
        gen_motion_yank(ctx, m, cmd_reps, args_reps, forced_mode)?;

    if m == Motion::NextBigWord || m == Motion::NextSubWord {
        adjust_for_c_cmd(ctx, cursor, &mut extent, &mut register_data);
        let orig_mode = motion_mode(m);
        let inclusive = is_inclusive(ctx, m, extent.overshot, orig_mode, forced_mode);
        let yank_shape = yank_shape(forced_mode.unwrap_or(orig_mode), extent, inclusive);
        let yank_data = YankData::new(extent.start, yank_shape);
        Some((register_data, yank_data))
    } else {
        Some((register_data, yank_data))
    }
}

// Generic motion-based yank
fn gen_motion_yank(
    ctx: &mut EditorCtx,
    m: Motion,
    cmd_reps: usize,
    args_reps: usize,
    forced_mode: Option<MotionMode>,
) -> Option<(RegisterData, MotionExtent, YankData)> {
    let (orig_cursor, orig_target_col) = {
        let (_, buf_view) = active_session!(ctx);
        (buf_view.cursor, buf_view.target_col)
    };

    let extent = exec_motion(ctx, m, cmd_reps, args_reps)?;
    let orig_mode = motion_mode(m);
    let inclusive = is_inclusive(ctx, m, extent.overshot, orig_mode, forced_mode);

    if extent.start < extent.end {
        let (_, buf_view) = active_session!(mut ctx);
        buf_view.cursor = orig_cursor;
        buf_view.target_col = orig_target_col;
    }

    let (register_data, yank_shape) = extent_yank(ctx, extent, orig_mode, forced_mode, inclusive);
    let yank_data = YankData::new(extent.start, yank_shape);
    Some((register_data, extent, yank_data))
}

// Yank text based on the given extent and mode
pub fn extent_yank(
    ctx: &mut EditorCtx,
    extent: MotionExtent,
    orig_mode: MotionMode,
    forced_mode: Option<MotionMode>,
    inclusive: bool,
) -> (RegisterData, YankShape) {
    let span = extent.to_ordered_span();
    let reg_data = match forced_mode.unwrap_or(orig_mode) {
        MotionMode::Charwise => {
            // We keep trailing '\n' in a charwise selection only if charwise is forced
            if forced_mode.is_some_and(|mode| mode == MotionMode::Charwise)
                && orig_mode != MotionMode::Charwise
            {
                select_charwise_nl(ctx, span, inclusive)
            } else {
                select_charwise(ctx, span, inclusive)
            }
        }
        MotionMode::Linewise => select_linewise(ctx, span),
        MotionMode::Blockwise => select_blockwise(ctx, span),
    };

    let yank_shape = yank_shape(forced_mode.unwrap_or(orig_mode), extent, inclusive);
    (reg_data, yank_shape)
}

// Adjust the given register data and motion extent to make is suitable for the 'c' command:
//
//    - charwise selection: give back trailing whitespace
//    - blockwise selection: give back trailing whitespace for each row
//    - linewise selection: do nothing
//
// We modify the given extent and register data accordingly.
fn adjust_for_c_cmd(
    ctx: &mut EditorCtx,
    cursor: Coords,
    extent: &mut MotionExtent,
    data: &mut RegisterData,
) {
    let (start, end) = extent.to_ordered_span();
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    match data {
        RegisterData::Char { data } => {
            let trimmed = data.trim_end();
            if !trimmed.is_empty() {
                data.truncate(trimmed.len());
                let start_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, start);
                let end_idx = start_idx + data.chars().count();
                extent.end = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, end_idx);
            }
        }
        RegisterData::Block { data, idxs } => {
            let start_row = start.row;
            for (i, (row, (start, end))) in data.iter_mut().zip(idxs).enumerate() {
                if row.chars().all(|c| c.is_whitespace()) {
                    continue;
                }
                adjust_row_for_c_cmd(ctx, start_row + i, row, *start, end);
            }
        }
        RegisterData::Line { .. } => {}
    }
}

fn adjust_row_for_c_cmd(
    ctx: &mut EditorCtx,
    row_num: usize,
    row_data: &mut String,
    start: usize,
    end: &mut usize,
) {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let line = buf_view
        .display_buf
        .ensure_line(&ctx.config, buffer.rope(), row_num);

    let line_idx = buffer.rope().line_to_char(row_num);
    let start_col = line.char_idx_to_col(start - line_idx);
    let mut end_col = line.char_idx_to_col(*end - line_idx);

    if let Some((_, span)) = line.grapheme_at(end_col.saturating_sub(1)) {
        let mut should_trim = false;
        let curr_col = span.start;
        let curr_idx = line_idx + line.col_to_char_idx(curr_col);

        if buffer.rope().char(curr_idx).is_whitespace() {
            should_trim = true;
            end_col = span.start;
        } else if span.start > start_col
            && line.grapheme_at(span.start - 1).is_some_and(|(g, span)| {
                let curr_col = span.start;
                let curr_idx = line_idx + line.col_to_char_idx(curr_col);
                buffer.rope().char(curr_idx).is_whitespace()
            })
        {
            should_trim = true;
            end_col = span.start;
        }

        if should_trim {
            let start_idx = line.col_to_char_idx(start_col);
            let end_idx = line.col_to_char_idx(end_col);
            row_data.truncate(end_idx - start_idx);

            let trimmed = row_data.trim_end();
            row_data.truncate(trimmed.len());
            *end = start + row_data.len();
        }
    }
}

fn yank_shape(mode: MotionMode, extent: MotionExtent, inclusive: bool) -> YankShape {
    let (start, end) = extent.to_ordered_span();
    let num_lines = end.row - start.row + 1;

    match mode {
        MotionMode::Charwise => YankShape::Char {
            num_lines,
            end_col: end.col,
            inclusive,
        },
        MotionMode::Linewise => YankShape::Line { num_lines },
        MotionMode::Blockwise => {
            let cols = start.col.max(end.col) - start.col.min(end.col) + 1;
            YankShape::Block {
                rows: num_lines,
                cols,
            }
        }
    }
}

fn motion_mode(m: Motion) -> MotionMode {
    if charwise(m) {
        MotionMode::Charwise
    } else {
        MotionMode::Linewise
    }
}

fn is_inclusive(
    ctx: &EditorCtx,
    m: Motion,
    overshot: bool,
    orig_mode: MotionMode,
    forced_mode: Option<MotionMode>,
) -> bool {
    let mut inclusive = inclusive(ctx, m) || (orig_mode == MotionMode::Charwise && overshot);
    if forced_mode.is_some_and(|mode| mode == MotionMode::Charwise) {
        inclusive = !inclusive;
    }
    inclusive
}

#[cfg(test)]
mod test {}
