use ropey::Rope;
use std::ops::Range;

use crate::{
    active_session, active_session_and_buffer,
    cmd::{Arg, Cmd, ImmediateOp, Motion, MotionMode},
    components::{Buffer, Coords, EditorCtx, MutBuffer, Register, RegisterData, Registers},
    systems::{
        commons::{char_idx_to_coords, coords_to_char_idx, curr_line},
        event,
        insert::{Damage, DamageEvent, broadcast_damage},
        nav::{
            NormalNav, charwise, exec_motion, goto_col, inclusive, select_blockwise,
            select_charwise, select_linewise, utils::ensure_cursor_inside_line,
        },
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
    if args.op != ImmediateOp::Join && is_readonly(args.cmd.reg) {
        return;
    }

    if args.op != ImmediateOp::Yank {
        ctx.repbuf.record_immediate(args.cmd);
    }

    let damage = match args.op {
        ImmediateOp::Delete => {
            let damage = delete(ctx, args.cmd.reg, args.cmd.reps.unwrap_or(1));
            ensure_cursor_inside_line(ctx);
            damage
        }
        ImmediateOp::Backspace => backspace(ctx, args.cmd.reg, args.cmd.reps.unwrap_or(1)),
        ImmediateOp::Join => join(ctx, args.cmd.reps.unwrap_or(1)),
        ImmediateOp::Yank => {
            yank(ctx, args.cmd);
            Damage::Intact
        }
    };

    let (session, _) = active_session!(ctx);
    let damage_evt = DamageEvent::new(session.buf_id, damage);
    broadcast_damage(ctx, damage_evt);
}

// -----------------------------------------------------------------------
// Rules for immediate updates
// Every immediate update must follow this sequence:
//
//  1. Update registers
//  2. Mutate the buffer's rope
//  3. Patch the active session's buffer view
//  4. Update cursor position
//  5. Compute and return the damage
// -----------------------------------------------------------------------

// -----------------------------------------------------------------------
// Small deletes
// -----------------------------------------------------------------------

// Delete chars at the current cursor position (x command)
fn delete(ctx: &mut EditorCtx, reg: Option<char>, reps: usize) -> Damage {
    let rng = calc_delete_range(ctx, reps);
    small_delete(ctx, reg, reps, rng)
}

// Delete chars behind the current cursor position (X command)
fn backspace(ctx: &mut EditorCtx, reg: Option<char>, reps: usize) -> Damage {
    let rng = calc_backspace_range(ctx, reps);
    small_delete(ctx, reg, reps, rng)
}

fn small_delete(ctx: &mut EditorCtx, reg: Option<char>, reps: usize, rng: Range<usize>) -> Damage {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    record_small_delete(&mut ctx.registers, reg, buffer.rope(), rng.clone());

    buffer.edit().remove(rng.clone());

    let row = buf_view.cursor.row;
    buf_view
        .display_buf
        .patch_range(&ctx.config, buffer.rope(), row..row + 1);

    let cursor = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, rng.start);
    goto_col::<NormalNav>(&ctx.config, buffer.rope(), buf_view, cursor.col);

    Damage::Line(buf_view.cursor.row)
}

fn calc_delete_range(ctx: &mut EditorCtx, reps: usize) -> Range<usize> {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let row = buf_view.cursor.row;
    let start_col = buf_view.cursor.col;
    let mut end_col = start_col;

    let line = curr_line(&ctx.config, buffer.rope(), buf_view);
    let mut it = line
        .graphemes_between(start_col, line.display_width)
        .enumerate();

    while let Some((i, (_, span))) = it.next() {
        if i >= reps {
            break;
        }
        end_col = span.end;
    }

    let start_coords = Coords::new(row, start_col);
    let end_coords = Coords::new(row, end_col);

    let start_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, start_coords);
    let end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, end_coords);

    start_idx..end_idx
}

fn calc_backspace_range(ctx: &mut EditorCtx, reps: usize) -> Range<usize> {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let row = buf_view.cursor.row;
    let end_col = buf_view.cursor.col;
    let mut start_col = end_col;

    let line = curr_line(&ctx.config, buffer.rope(), buf_view);
    let mut it = line.rev_graphemes_between(start_col, 0).enumerate();

    while let Some((i, (g, span))) = it.next() {
        if i >= reps {
            break;
        }
        start_col = span.start;
    }

    let start_coords = Coords::new(row, start_col);
    let end_coords = Coords::new(row, end_col);

    let start_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, start_coords);
    let end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, end_coords);

    start_idx..end_idx
}

// Store small delete in registers
fn record_small_delete(
    registers: &mut Registers,
    reg: Option<char>,
    rope: &Rope,
    range: Range<usize>,
) {
    let deleted = rope.slice(range);
    registers.record_small_delete(reg, deleted);
}

// -----------------------------------------------------------------------
// Join N lines (J command)
// -----------------------------------------------------------------------
fn join(ctx: &mut EditorCtx, reps: usize) -> Damage {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let row = buf_view.cursor.row;
    if row + 1 == buffer.rope().len_lines() {
        return Damage::Intact;
    }

    let mut reps = reps.saturating_sub(1);
    let mut cursor_idx;

    loop {
        cursor_idx = join_single(buffer, row);

        reps = reps.saturating_sub(1);
        if reps == 0 || row + 1 == buffer.rope().len_lines() {
            break;
        }
    }

    buf_view.display_buf.destroy_from(row);
    buf_view.cursor = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, cursor_idx);

    Damage::From(row)
}

fn join_single(buffer: &mut Buffer, row: usize) -> usize {
    let boundary = buffer.rope().line_to_char(row + 1);

    let mut start_idx = boundary - 1;
    if start_idx > 0
        && buffer.rope().char(start_idx) == '\n'
        && buffer.rope().char(start_idx - 1) == '\r'
    {
        start_idx -= 1;
    }

    let end_idx = buffer
        .rope()
        .chars_at(boundary)
        .position(|c| !c.is_whitespace() || c == '\n' || c == '\r')
        .map(|pos| boundary + pos)
        .unwrap_or(buffer.rope().len_chars());

    buffer.edit().remove(start_idx..end_idx);

    if start_idx > 0
        && buffer.rope().char(start_idx) != '\r'
        && buffer.rope().char(start_idx) != '\n'
    {
        match buffer.rope().char(start_idx - 1) {
            c if c.is_whitespace() => {}
            c if c == '.' || c == '?' || c == '!' => buffer.edit().insert(start_idx, "  "),
            _ => buffer.edit().insert_char(start_idx, ' '),
        }
    }

    let cursor_idx =
        if buffer.rope().char(start_idx) == '\r' || buffer.rope().char(start_idx) == '\n' {
            start_idx.saturating_sub(1)
        } else {
            start_idx
        };

    cursor_idx
}

// -----------------------------------------------------------------------
// Yank
// -----------------------------------------------------------------------
pub fn yank(ctx: &mut EditorCtx, cmd: Cmd) {
    match cmd.arg {
        Arg::Motion { reps, motion } => {
            let cmd_reps = cmd.reps.unwrap_or(1);
            let arg_reps = reps.unwrap_or(1);
            // TODO get forced_mode from cmd
            match motion_yank(ctx, motion, cmd_reps, arg_reps, None) {
                None => {}
                Some(reg_data) => {
                    event::on_yank(&mut ctx.status, &reg_data);
                    ctx.registers.record_yank(cmd.reg, reg_data);
                }
            }
        }
        Arg::TextObject { .. } => {}
        Arg::None => {}
    };
}

fn motion_yank(
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

// -----------------------------------------------------------------------
// Auxiliary stuff from now on
// -----------------------------------------------------------------------
fn is_readonly(reg: Option<char>) -> bool {
    reg.map(Register::from).is_some_and(|r| r.is_readonly())
}
