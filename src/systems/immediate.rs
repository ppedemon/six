use std::ops::Range;

use ropey::Rope;

use crate::{
    cmd::{Cmd, ImmediateOp},
    components::{Coords, EditorCtx, Register, Registers},
    systems::{
        commons::{char_idx_to_coords, coords_to_char_idx, curr_line},
        insert::{Damage, DamageEvent, broadcast_damage},
        nav::{NormalNav, goto_col, utils::ensure_cursor_inside_line},
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
    if is_readonly(args.cmd.reg) {
        return;
    }

    ctx.repbuf.record_immediate(args.cmd);

    let damage = match args.op {
        ImmediateOp::Delete => {
            let damage = delete(ctx, args.cmd.reg, args.cmd.reps.unwrap_or(1));
            ensure_cursor_inside_line(ctx);
            damage
        }
        ImmediateOp::Backspace => backspace(ctx, args.cmd.reg, args.cmd.reps.unwrap_or(1)),
    };

    let (session, _) = ctx.active_session();
    let damage_evt = DamageEvent::new(session.buf_id, damage);
    broadcast_damage(ctx, damage_evt);
}

// -----------------------------------------------------------------------
// Rules for immediate updates:
// Every immediate update must follow this sequence:
//
//  1. Update registers
//  2. Mutate the buffer's rope
//  3. Mark the buffer as dirty
//  4. Patch the active session's buffer view
//  5. Update cursor position
//  6. Compute and return the damage
// -----------------------------------------------------------------------

//
// Delete chars at the current cursor position on the given line
//
fn delete(ctx: &mut EditorCtx, reg: Option<char>, reps: usize) -> Damage {
    let (session, buf_view) = ctx.sessions.get_mut(&ctx.editor.session_id).unwrap();
    let buffer = ctx.buffers.get_mut(&session.buf_id).unwrap();

    let row = buf_view.cursor.row;
    let start_col = buf_view.cursor.col;
    let mut end_col = start_col;

    let line = curr_line(&ctx.config, &buffer.rope, buf_view);
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

    let start_idx = coords_to_char_idx(&ctx.config, &buffer.rope, buf_view, start_coords);
    let end_idx = coords_to_char_idx(&ctx.config, &buffer.rope, buf_view, end_coords);

    report_small_delete(&mut ctx.registers, reg, &buffer.rope, start_idx..end_idx);

    buffer.rope.remove(start_idx..end_idx);
    buffer.dirty = true;

    buf_view
        .display_buf
        .patch_range(&ctx.config, &buffer.rope, row..row + 1);

    let cursor = char_idx_to_coords(&ctx.config, &buffer.rope, buf_view, start_idx);
    goto_col::<NormalNav>(&ctx.config, &buffer.rope, buf_view, cursor.col);

    Damage::Line(buf_view.cursor.row)
}

//
// Delete chars at the position behond the cursor on the given line
//
fn backspace(ctx: &mut EditorCtx, reg: Option<char>, reps: usize) -> Damage {
    let (session, buf_view) = ctx.sessions.get_mut(&ctx.editor.session_id).unwrap();
    let buffer = ctx.buffers.get_mut(&session.buf_id).unwrap();

    let row = buf_view.cursor.row;
    let end_col = buf_view.cursor.col;
    let mut start_col = end_col;

    let line = curr_line(&ctx.config, &buffer.rope, buf_view);
    let mut it = line.rev_graphemes_between(start_col, 0).enumerate();

    while let Some((i, (g, span))) = it.next() {
        if i >= reps {
            break;
        }
        start_col = span.start;
    }

    let start_coords = Coords::new(row, start_col);
    let end_coords = Coords::new(row, end_col);

    let start_idx = coords_to_char_idx(&ctx.config, &buffer.rope, buf_view, start_coords);
    let end_idx = coords_to_char_idx(&ctx.config, &buffer.rope, buf_view, end_coords);

    report_small_delete(&mut ctx.registers, reg, &buffer.rope, start_idx..end_idx);

    buffer.rope.remove(start_idx..end_idx);
    buffer.dirty = true;

    buf_view
        .display_buf
        .patch_range(&ctx.config, &buffer.rope, row..row + 1);

    let cursor = char_idx_to_coords(&ctx.config, &buffer.rope, buf_view, start_idx);
    goto_col::<NormalNav>(&ctx.config, &buffer.rope, buf_view, cursor.col);

    Damage::Line(buf_view.cursor.row)
}

fn is_readonly(reg: Option<char>) -> bool {
    reg.map(Register::from).is_some_and(|r| r.is_readonly())
}

fn report_small_delete(
    registers: &mut Registers,
    reg: Option<char>,
    rope: &Rope,
    range: Range<usize>,
) {
    let deleted = rope.slice(range);
    registers.small_delete(reg, deleted);
}
