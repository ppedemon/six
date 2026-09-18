use std::sync::LazyLock;

use crate::{
    active_session, active_session_and_buffer,
    cmd::{
        Cmd, EditOp, InsertPoint, InteractiveOp, Motion,
        Operator::{self, Move},
        TxnItem,
    },
    components::{EditorCtx, YankData, YankShape},
    systems::{
        commons::curr_line,
        immediate::delete_for_c_cmd,
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

fn change_prelude<'a>(ctx: &mut EditorCtx, args: &'a InteractiveArgs) {
    let (row, num_rows) = {
        let (_, buf_view, buffer) = active_session_and_buffer!(ctx);
        (buf_view.cursor.row, buffer.rope().len_lines())
    };

    delete_for_c_cmd(ctx, args.cmd);

    if let Some(shape) = ctx.repbuf.last_yank() {
        match shape {
            YankData {
                shape: YankShape::Line { num_lines },
                ..
            } if num_lines < num_rows => {
                if row + num_lines >= num_rows {
                    dispatch_txn(ctx, &OPEN_BELOW)
                } else {
                    dispatch_txn(ctx, &OPEN_ABOVE)
                }
            }
            _ => {}
        }
    }
}

pub fn finish_interactive(ctx: &mut EditorCtx, op: InteractiveOp, reps: usize) {
    if op == InteractiveOp::Change {
        finish_interactive_change(ctx);
    } else {
        apply_last_insert(ctx, op, reps, false);
    }
}

fn finish_interactive_change(ctx: &mut EditorCtx) {
    match ctx.repbuf.last_yank() {
        Some(YankData {
            shape: YankShape::Block { rows, .. },
            start,
        }) => {
            let (_, buf_view) = active_session!(ctx);
            let col = start.col;
            let ops = ctx.registers.last_insert().to_vec();

            for _ in 0..rows.saturating_sub(1) {
                dispatch_txn(
                    ctx,
                    &[
                        Cmd::new(Operator::Move(Motion::Down)).into(),
                        Cmd::new(Operator::Move(Motion::GotoCol(col + 1))).into(),
                    ],
                );
                apply_insert_log(ctx, &ops, 1);
            }

            let (_, buf_view) = active_session!(ctx);
            let cursor_col = buf_view.cursor.col;
            dispatch_txn(
                ctx,
                &[
                    Cmd::new(Operator::Move(Motion::GotoLine(start.row + 1))).into(),
                    Cmd::new(Operator::Move(Motion::GotoCol(cursor_col + 1))).into(),
                ],
            );
        }
        _ => {}
    }
}

fn insert_point(ctx: &mut EditorCtx, op: InteractiveOp) -> InsertPoint {
    match op {
        InteractiveOp::EnterInsert(insert_point) => insert_point,
        InteractiveOp::Change => {
            let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
            let cursor = buf_view.cursor;
            let line = curr_line(&ctx.config, buffer.rope(), buf_view);
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
            if let Some(yank_data) = ctx.repbuf.last_yank() {
                match yank_data {
                    YankData {
                        shape: YankShape::Block { rows, .. },
                        ..
                    } => {
                        let ops = ctx.registers.last_insert().to_vec();
                        apply_insert_log(ctx, &ops, 1);
                        for _ in 0..rows.saturating_sub(1) {
                            dispatch_cmd(ctx, Cmd::new(Operator::Move(Motion::Down)));
                            apply_insert_log(ctx, &ops, 1);
                        }
                    }
                    _ => {
                        let ops = ctx.registers.last_insert().to_vec();
                        apply_insert_log(ctx, &ops, reps);
                    }
                }
            }
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
