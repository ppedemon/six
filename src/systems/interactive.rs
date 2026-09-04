use std::{borrow::Cow, sync::LazyLock};

use crate::{
    cmd::{Cmd, EditOp, InsertPoint, InteractiveOp, Motion, Operator::Move, TxnItem},
    components::EditorCtx,
    systems::{
        enter_insert, input::dispatch_txn, insert::apply_insert_log, sys::goto_insert_point,
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

pub struct InteractiveArgs {
    op: InteractiveOp,
    cmd: Cmd,
}

impl InteractiveArgs {
    pub fn new(op: InteractiveOp, cmd: Cmd) -> Self {
        Self { op, cmd }
    }
}

pub fn handle_interactive(ctx: &mut EditorCtx, args: InteractiveArgs) {
    ctx.repbuf.save_last_cmd(args.cmd);
    exec_interactive(ctx, args);
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

fn exec_interactive(ctx: &mut EditorCtx, args: InteractiveArgs) {
    let txn = prelude_txn(&args);
    let insert_point = insert_point(args.op);
    dispatch_txn(ctx, &txn);
    enter_insert(ctx, insert_point, args.cmd);
}

pub fn exec_prologue(ctx: &mut EditorCtx, op: InteractiveOp, reps: usize) {
    match op {
        InteractiveOp::EnterInsert(_) => {
            let ops = ctx.registers.last_insert().to_vec();
            apply_insert_log(ctx, &ops, reps);
        }
        InteractiveOp::OpenAbove | InteractiveOp::OpenBelow => {
            let ops = ctx.registers.last_insert();
            let mut new_ops = Vec::with_capacity(ops.len() + 1);
            new_ops.push(EditOp::Enter);
            new_ops.extend(ops);
            apply_insert_log(ctx, &new_ops, reps);
        }
    }
}

pub fn exec_batch(ctx: &mut EditorCtx, args: InteractiveArgs) {
    let reps = args.cmd.reps.unwrap_or(1);
    let txn = prelude_txn(&args);
    let insert_point = insert_point(args.op);
    dispatch_txn(ctx, &txn);
    goto_insert_point(ctx, insert_point);
    exec_prologue(ctx, args.op, reps);
}
