use std::sync::LazyLock;

use crate::{
    active_session_and_buffer,
    cmd::{
        Arg, Cmd, EditOp, ImmediateOp, InsertPoint, InteractiveOp, Motion,
        Operator::{self, Move},
        TxnItem,
    },
    components::{EditorCtx, YankShape},
    systems::{
        input::{dispatch_cmd, dispatch_txn},
        insert::apply_insert_log,
        interactive::ExecMode::{Batch, Interactive},
        mode::{enter_insert, goto_insert_point},
    },
};

static OPEN_ABOVE: LazyLock<Vec<TxnItem>> = LazyLock::new(|| {
    let txn = vec![
        Cmd::new(Move(Motion::StartOfLine)).into(),
        EditOp::Enter.into(),
        Cmd::new(Move(Motion::Up)).into(),
    ];
    txn
});

static OPEN_BELOW: LazyLock<Vec<TxnItem>> = LazyLock::new(|| {
    let txn = vec![
        Cmd::new(Move(Motion::EndOfLinePlusOne)).into(),
        EditOp::Enter.into(),
    ];
    txn
});

enum ExecMode {
    Interactive,
    Batch,
}

pub struct InteractiveArgs {
    op: InteractiveOp,
    cmd: Cmd,
    exec_mode: ExecMode,
}

impl InteractiveArgs {
    pub fn new(op: InteractiveOp, cmd: Cmd) -> Self {
        Self {
            op,
            cmd,
            exec_mode: Interactive,
        }
    }

    pub fn batch(op: InteractiveOp, cmd: Cmd) -> Self {
        Self {
            op,
            cmd,
            exec_mode: Batch,
        }
    }
}

pub fn handle_interactive(ctx: &mut EditorCtx, args: InteractiveArgs) {
    prelude(ctx, &args);
    ctx.repbuf.save_last_cmd(args.cmd);
    let insert_point = insert_point(ctx, args.op);

    match args.exec_mode {
        ExecMode::Interactive => enter_insert(ctx, insert_point),
        ExecMode::Batch => {
            let reps = args.cmd.reps.unwrap_or(1);
            goto_insert_point(ctx, insert_point);
            apply_last_insert(ctx, args.op, reps, true);
        }
    }
}

fn prelude(ctx: &mut EditorCtx, args: &InteractiveArgs) {
    match args.op {
        InteractiveOp::EnterInsert(insert_point) => {}
        InteractiveOp::OpenAbove => dispatch_txn(ctx, &OPEN_ABOVE),
        InteractiveOp::OpenBelow => dispatch_txn(ctx, &OPEN_BELOW),
        InteractiveOp::Change => change_prelude(ctx, args),
    }
}

pub fn finish_interactive(ctx: &mut EditorCtx, op: InteractiveOp, reps: usize) {
    let reps = if op == InteractiveOp::Change { 1 } else { reps };
    apply_last_insert(ctx, op, reps, false);
}

fn change_prelude<'a>(ctx: &mut EditorCtx, args: &'a InteractiveArgs) {
    let arg = match args.cmd.arg {
        Arg::Motion { reps, mode, motion } => match motion {
            Motion::NextBigWord => Arg::motion(reps, mode, Motion::EndBigWord),
            Motion::NextSubWord => Arg::motion(reps, mode, Motion::EndSubWord),
            _ => args.cmd.arg,
        },
        arg => arg,
    };
    let cmd = Cmd::new(Operator::Immediate(ImmediateOp::Delete))
        .reps(args.cmd.reps)
        .reg(args.cmd.reg)
        .arg(arg);
    dispatch_cmd(ctx, cmd);

    if let Some(yank_shape) = ctx.repbuf.last_yank_shape() {
        match yank_shape {
            YankShape::Line { .. } => dispatch_txn(ctx, &OPEN_ABOVE),
            _ => {}
        }
    }
}

fn insert_point(ctx: &mut EditorCtx, op: InteractiveOp) -> InsertPoint {
    match op {
        InteractiveOp::EnterInsert(insert_point) => insert_point,
        InteractiveOp::Change => {
            let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
            let cursor = buf_view.cursor;
            let line = buf_view
                .display_buf
                .ensure_line(&ctx.config, buffer.rope(), cursor.row);
            if line.next_col(cursor.col) == cursor.col {
                InsertPoint::Last
            } else {
                InsertPoint::Curr
            }
        }
        _ => InsertPoint::Curr,
    }
}

fn apply_last_insert(ctx: &mut EditorCtx, op: InteractiveOp, reps: usize, from_batch: bool) {
    match op {
        InteractiveOp::EnterInsert(_) => {
            let ops = ctx.registers.last_insert().to_vec();
            apply_insert_log(ctx, &ops, reps);
        }
        InteractiveOp::Change => {
            let ops = ctx.registers.last_insert().to_vec();
            apply_insert_log(ctx, &ops, 1);
        }
        InteractiveOp::OpenAbove | InteractiveOp::OpenBelow => {
            let len = ctx.registers.last_insert().len();
            let mut new_ops = Vec::with_capacity(len + 1);
            new_ops.push(EditOp::Enter);
            new_ops.extend(ctx.registers.last_insert());
            let mut n = reps;
            if from_batch {
                apply_insert_log(ctx, &new_ops[1..], 1);
                n -= 1;
            }
            apply_insert_log(ctx, &new_ops, n);
        }
    }
}
