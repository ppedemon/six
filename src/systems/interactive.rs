use std::sync::LazyLock;

use crate::{
    active_session, active_session_and_buffer,
    cmd::{
        Cmd, EditOp, InsertPoint, InteractiveOp, Motion, MotionMode,
        Operator::{self, Move},
        TxnItem,
    },
    components::{EditorCtx, YankData},
    systems::{
        commons::{display_width, next_col_or_display_width},
        immediate::delete::delete,
        input::dispatch_txn,
        insert::{DamageEvent, apply_insert_log, broadcast_damage},
        interactive::ExecMode::{Batch, Interactive},
        mode::{enter_insert, goto_insert_point},
        nav::utils::ensure_cursor_inside_line,
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

fn change_prelude(ctx: &mut EditorCtx, args: &InteractiveArgs) {
    let (buf_id, row, num_rows) = {
        let (session, buf_view, buffer) = active_session_and_buffer!(ctx);
        (
            session.buf_id,
            buf_view.cursor.row,
            buffer.rope().len_lines(),
        )
    };

    let damage = delete(ctx, args.cmd);
    broadcast_damage(ctx, DamageEvent::new(buf_id, damage));

    if let Some(yank_data) = ctx.repbuf.last_yank() {
        match yank_data.mode {
            MotionMode::Linewise => {
                let num_lines = yank_data.num_lines();
                if num_lines < num_rows {
                    if row + num_lines >= num_rows {
                        dispatch_txn(ctx, &OPEN_BELOW)
                    } else {
                        dispatch_txn(ctx, &OPEN_ABOVE)
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn finish_interactive(ctx: &mut EditorCtx, op: InteractiveOp, reps: usize) {
    apply_last_insert(ctx, op, reps, false);
}

fn insert_point(ctx: &mut EditorCtx, op: InteractiveOp) -> InsertPoint {
    match op {
        InteractiveOp::EnterInsert(insert_point) => insert_point,
        InteractiveOp::Change => {
            let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
            let rope = buffer.rope();
            let cursor = buf_view.cursor;

            let dw = display_width(&ctx.config, rope, buf_view, cursor);
            if next_col_or_display_width(&ctx.config, buffer.rope(), buf_view, cursor) == dw {
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
            apply_change_insert(ctx, from_batch);
        }
        InteractiveOp::OpenAbove | InteractiveOp::OpenBelow => {
            apply_open_insert(ctx, reps, from_batch)
        }
    }

    if from_batch {
        ensure_cursor_inside_line(ctx);
    }
}

fn apply_change_insert(ctx: &mut EditorCtx, from_batch: bool) {
    let ops = ctx.registers.last_insert().to_vec();
    if from_batch {
        apply_insert_log(ctx, &ops, 1);
        ctx.status.clear_msg(); // Batch change commands don't show delete notifications
    }

    match ctx.repbuf.last_yank() {
        Some(
            yank_data @ YankData {
                mode: MotionMode::Blockwise,
                start,
                ..
            },
        ) => {
            let (_, buf_view) = active_session!(ctx);
            let cursor = buf_view.cursor;
            let rows = yank_data.num_lines();

            if cursor.row != start.row {
                return;
            }

            for _ in 0..rows.saturating_sub(1) {
                dispatch_txn(
                    ctx,
                    &[
                        Cmd::new(Operator::Move(Motion::Down)).into(),
                        // Note: GotoCol is one-based. First col is #1.
                        Cmd::new(Operator::Move(Motion::GotoCol(start.col + 1))).into(),
                    ],
                );
                apply_insert_log(ctx, &ops, 1);
            }

            dispatch_txn(
                ctx,
                &[
                    Cmd::new(Operator::Move(Motion::GotoLine(cursor.row + 1))).into(),
                    Cmd::new(Operator::Move(Motion::GotoCol(cursor.col))).into(),
                ],
            );
        }
        _ => {}
    }
}

fn apply_open_insert(ctx: &mut EditorCtx, reps: usize, from_batch: bool) {
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
