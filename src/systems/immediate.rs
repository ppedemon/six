use crate::{
    active_session,
    cmd::{Arg, Cmd, ImmediateOp, Motion},
    components::{EditorCtx, Register},
    systems::{
        immediate::delete::delete,
        insert::{Damage, DamageEvent, broadcast_damage},
        nav::utils::ensure_cursor_inside_line,
    },
};

mod delete;
mod delete_char;
mod join;
mod paste;
mod yank;

use delete_char::{backspace, delete_char};
use join::join;
use paste::{PasteMode, paste};
use yank::yank;

#[derive(Clone, Copy)]
pub struct ImmediateArgs {
    op: ImmediateOp,
    cmd: Cmd,
}

impl ImmediateArgs {
    pub fn new(op: ImmediateOp, cmd: Cmd) -> Self {
        Self { op, cmd }
    }
}

// -----------------------------------------------------------------------
// Rules for immediate updates
// Every immediate update must follow this sequence:
//
//  1. Update registers if required
//  2. Mutate the buffer's rope if required
//  3. Patch the active session's buffer view
//  4. Update cursor position
//  5. Compute and return the damage
// -----------------------------------------------------------------------

pub fn handle_immediate(ctx: &mut EditorCtx, args: ImmediateArgs) {
    if updates_registers(args.op) && is_readonly(args.cmd.reg) {
        return;
    }

    if is_repeatable(args.op) {
        ctx.repbuf.save_last_cmd(args.cmd);
    }

    let damage = match args.op {
        ImmediateOp::DeleteChar => {
            let damage = delete_char(ctx, args.cmd.reg, args.cmd.reps.unwrap_or(1));
            ensure_cursor_inside_line(ctx);
            damage
        }
        ImmediateOp::Backspace => backspace(ctx, args.cmd.reg, args.cmd.reps.unwrap_or(1)),
        ImmediateOp::Join => join(ctx, args.cmd.reps.unwrap_or(1)),
        ImmediateOp::Yank => {
            yank(ctx, args.cmd);
            Damage::Intact
        }
        ImmediateOp::YankLine => {
            let mut fake_args = args;
            fake_args.cmd = fake_args.cmd.arg(Arg::motion(None, None, Motion::Line));
            yank(ctx, fake_args.cmd);
            Damage::Intact
        }
        ImmediateOp::Paste => paste(ctx, args.cmd, PasteMode::After),
        ImmediateOp::PasteBefore => paste(ctx, args.cmd, PasteMode::Before),
        ImmediateOp::Delete => delete(ctx, args.cmd),
    };

    let (session, _) = active_session!(ctx);
    let damage_evt = DamageEvent::new(session.buf_id, damage);
    broadcast_damage(ctx, damage_evt);
}

// -----------------------------------------------------------------------
// Auxiliary stuff from now on
// -----------------------------------------------------------------------
fn is_readonly(reg: Option<char>) -> bool {
    reg.map(Register::from).is_some_and(|r| r.is_readonly())
}

fn is_repeatable(op: ImmediateOp) -> bool {
    match op {
        ImmediateOp::Yank | ImmediateOp::YankLine => false,
        _ => true,
    }
}

fn updates_registers(op: ImmediateOp) -> bool {
    match op {
        ImmediateOp::Join | ImmediateOp::Paste | ImmediateOp::PasteBefore => false,
        _ => true,
    }
}
