use std::{borrow::Cow, sync::LazyLock};

use crate::{
    active_session_and_buffer,
    cmd::{Cmd, EditOp, InsertPoint, InteractiveOp, Motion, Operator::Move, TxnItem},
    components::EditorCtx,
    systems::{
        commons,
        input::dispatch_txn,
        insert::apply_insert_log,
        interactive::ExecMode::{Batch, Interactive},
        mode::{enter_insert, goto_insert_point},
        nav::{NormalNav, move_left},
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
    ctx.repbuf.save_last_cmd(args.cmd);
    match args.exec_mode {
        ExecMode::Interactive => exec_interactive(ctx, args),
        ExecMode::Batch => exec_batch(ctx, args),
    }
}

fn exec_interactive(ctx: &mut EditorCtx, args: InteractiveArgs) {
    let txn = prelude_txn(&args);
    let insert_point = insert_point(args.op);
    dispatch_txn(ctx, &txn);
    enter_insert(ctx, insert_point);
}

fn exec_batch(ctx: &mut EditorCtx, args: InteractiveArgs) {
    let reps = args.cmd.reps.unwrap_or(1);
    let txn = prelude_txn(&args);
    let insert_point = insert_point(args.op);
    dispatch_txn(ctx, &txn);
    goto_insert_point(ctx, insert_point);
    apply_last_insert(ctx, args.op, reps, true);
}

pub fn finish_interactive(ctx: &mut EditorCtx, op: InteractiveOp, reps: usize) {
    apply_last_insert(ctx, op, reps, false);
}

fn prelude_txn<'a>(args: &'a InteractiveArgs) -> Cow<'a, [TxnItem]> {
    match args.op {
        InteractiveOp::EnterInsert(_) => Cow::Borrowed(&[]),
        InteractiveOp::OpenAbove => Cow::Borrowed(OPEN_ABOVE.as_slice()),
        InteractiveOp::OpenBelow => Cow::Borrowed(OPEN_BELOW.as_slice()),
    }
}

fn insert_point(op: InteractiveOp) -> InsertPoint {
    match op {
        InteractiveOp::EnterInsert(insert_point) => insert_point,
        _ => InsertPoint::Curr,
    }
}

fn restore_cursor(ctx: &mut EditorCtx) {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let cursor = buf_view.cursor;
    let line = commons::curr_line(&ctx.config, buffer.rope(), buf_view);
    buf_view.cursor.col = line.snap_col(cursor.col);
    move_left::<NormalNav>(&ctx.config, buffer.rope(), buf_view, 1);
}

fn apply_last_insert(ctx: &mut EditorCtx, op: InteractiveOp, reps: usize, from_batch: bool) {
    match op {
        InteractiveOp::EnterInsert(_) => {
            let ops = ctx.registers.last_insert().to_vec();
            apply_insert_log(ctx, &ops, reps);
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
    restore_cursor(ctx);
}
