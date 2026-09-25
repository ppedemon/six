use std::{assert_ne, unimplemented};

use ropey::Rope;

use crate::{
    active_session, active_session_and_buffer,
    cmd::{Arg, Cmd, ImmediateOp, InteractiveOp, Motion, MotionMeta, MotionMode, Operator},
    components::{BufferView, Config, Coords, EditorCtx, LastSearch, RegisterData, YankData},
    systems::{
        commons::{
            char_idx_to_coords, coords_to_char_idx, cursor_to_char_idx, display_line,
            next_col_or_display_width,
        },
        input::dispatch_cmd,
    },
};

// The cursor range produced by applying a motion, including whether the
// motion reached beyond the available text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Extent {
    start: Coords,
    end: Coords,
    overshot: bool,
}

/// Interprets `cmd` as an argument-taking navigation command and applies its
/// effect to the editor state in `ctx`, which contains the active session.
pub fn interpret(ctx: &mut EditorCtx, cmd: Cmd) -> (YankData, RegisterData) {
    match cmd.arg {
        Arg::None => {
            // TODO None could be used for motions in visual mode. In this case,
            // We could grab YankData from the clipboard (to be added), or if
            // data not there, from the last YankData in the repbuf.
            unimplemented!("Implement arg-less motions (visual mode)")
        }
        Arg::Motion { reps, mode, motion } => {
            // Viewport motion change the viewport rather than moving the cursor
            assert!(motion.meta() != MotionMeta::Viewport);

            let arg_reps = cmd.reps.unwrap_or(1).saturating_mul(reps.unwrap_or(1));
            interpret_motion(ctx, cmd.op, motion, mode, arg_reps)
        }
        Arg::TextObject {
            reps,
            mode,
            text_object,
        } => {
            // TODO Implement a function computing (YankData, RegisterData) from
            // a text object, and apply logic similar to interpret_motion. We can
            // probably reuse a lot of code from interpret_motion.
            unimplemented!("Implement text-object args")
        }
    }
}

// Applies motion `m` for an operator, returning the text range it selects.
//
// `ctx` provides the active session and receives the resulting cursor
// changes, `op` determines operator-specific motion semantics, `given_mode`
// optionally overrides the motion's natural selection mode, and `arg_reps`
// is the number of times to apply the motion.
fn interpret_motion(
    ctx: &mut EditorCtx,
    op: Operator,
    m: Motion,
    given_mode: Option<MotionMode>,
    arg_reps: usize,
) -> (YankData, RegisterData) {
    let mut extent = {
        let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
        let m = fix_c(&ctx.config, buf_view, buffer.rope(), op, m);
        exec_motion(ctx, m, arg_reps)
    };

    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    if extent.start > extent.end {
        std::mem::swap(&mut extent.start, &mut extent.end);
    } else {
        buf_view.cursor = extent.start;
        buf_view.target_col = extent.start.col;
    }

    let inclusive = {
        let orig_inclusive = extent.overshot || inclusive(&ctx.last_search, m);
        match given_mode {
            Some(MotionMode::Charwise) => !orig_inclusive,
            Some(MotionMode::Linewise) | Some(MotionMode::Blockwise) => true,
            None => orig_inclusive,
        }
    };

    let mode = given_mode.unwrap_or(motion_mode(m));
    let mut yank_data = YankData::new(extent.start, extent.end, mode, inclusive);

    if is_w(m) && buffer.rope().len_chars() > 0 {
        fix_w(ctx, m, &mut yank_data);
    }

    apply_exceptions(ctx, &mut yank_data);
    if op == Operator::Immediate(ImmediateOp::Delete) && mode == MotionMode::Charwise {
        fix_d(ctx, &mut yank_data);
    }

    let reg_data = match yank_data.mode {
        MotionMode::Charwise => yank_charwise(ctx, yank_data),
        MotionMode::Linewise => yank_linewise(ctx, yank_data),
        MotionMode::Blockwise => yank_blockwise(ctx, yank_data),
    };

    (yank_data, reg_data)
}

// Adjusts the given motion for operator-specific change behavior.
//
// `config`, `buf_view`, and `rope` describe the active buffer, `op` is the
// operator being applied, and `m` is the requested motion.
fn fix_c(
    config: &Config,
    buf_view: &mut BufferView,
    rope: &Rope,
    op: Operator,
    m: Motion,
) -> Motion {
    match m {
        Motion::NextBigWord if op == Operator::Interactive(InteractiveOp::Change) => {
            if cursor_on_whitespace(config, buf_view, rope) {
                m
            } else {
                Motion::EndBigWord
            }
        }
        Motion::NextSubWord if op == Operator::Interactive(InteractiveOp::Change) => {
            if cursor_on_whitespace(config, buf_view, rope) {
                m
            } else {
                Motion::EndBigWord
            }
        }
        _ => m,
    }
}

// Executes `m` `arg_reps` times and returns the cursor range it traversed.
//
// `ctx` is updated with the resulting cursor position. Viewport-only motions
// do not select text and therefore return `None`.
fn exec_motion(ctx: &mut EditorCtx, m: Motion, arg_reps: usize) -> Extent {
    assert!(m.meta() != MotionMeta::Viewport);

    let (_, buf_view) = active_session!(ctx);
    let start = buf_view.cursor;

    let cmd = Cmd::new(Operator::Move(m)).reps(Some(arg_reps));
    dispatch_cmd(ctx, cmd);

    let (_, buf_view) = active_session!(ctx);
    let end = buf_view.cursor;

    Extent {
        start,
        end,
        overshot: buf_view.overshot,
    }
}

// Corrects a forward word-motion range that ends in leading whitespace.
//
// `ctx` provides the active buffer and display configuration, `m` identifies
// the original motion, and `yank_data` receives the corrected range.
fn fix_w(ctx: &mut EditorCtx, m: Motion, yank_data: &mut YankData) {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, yank_data.end);
    let line_idx = buffer.rope().line_to_char(yank_data.end.row);

    if buffer
        .rope()
        .slice(line_idx..end_idx)
        .chars()
        .all(|c| c.is_whitespace())
    {
        let end_idx = line_idx.saturating_sub(2);
        yank_data.end = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, end_idx);
        yank_data.inclusive = true;
    }
}

// Returns whether `m` is a forward big-word or sub-word motion.
fn is_w(m: Motion) -> bool {
    m == Motion::NextBigWord || m == Motion::NextSubWord
}

// Returns whether the active cursor in `buf_view` points at whitespace.
//
// `config` supplies display settings and `rope` is the buffer text used to
// resolve the cursor position.
fn cursor_on_whitespace(config: &Config, buf_view: &mut BufferView, rope: &Rope) -> bool {
    let char_idx = cursor_to_char_idx(config, buf_view, rope);
    rope.char(char_idx).is_whitespace()
}

// Reports whether a selection made by `m` includes its endpoint.
//
// `ctx` provides prior character-search state for repeated-search motions.
fn inclusive(last_search: &LastSearch, m: Motion) -> bool {
    match m {
        _ if m.meta() == MotionMeta::Linewise => true,

        Motion::EndOfLine => true,
        Motion::FindNextChar(_) => true,
        Motion::TillNextChar(_) => true,
        Motion::EndSubWord => true,
        Motion::EndBigWord => true,
        Motion::RepeatBackward => last_search
            .last_char_search()
            .is_some_and(|m| !inclusive(last_search, m)),
        Motion::RepeatForward => last_search
            .last_char_search()
            .is_some_and(|m| inclusive(last_search, m)),

        _ => false,
    }
}

// Maps `m` to the selection mode it naturally produces.
//
// Viewport-only motions are invalid because they do not describe a text
// range.
fn motion_mode(m: Motion) -> MotionMode {
    let meta = m.meta();
    assert_ne!(meta, MotionMeta::Viewport);

    if m.meta() == MotionMeta::Charwise {
        MotionMode::Charwise
    } else {
        MotionMode::Linewise
    }
}

// Applies exclusive/linewise and exclusive/inclusive excetion rules for
// ranges that end at the beginning of a later line.
//
// `ctx` provides the active buffer and display configuration, while
// `yank_data` is updated with the corrected range, inclusivity, and mode.
fn apply_exceptions(ctx: &mut EditorCtx, yank_data: &mut YankData) {
    if !yank_data.inclusive && yank_data.end.col == 0 && yank_data.start.row < yank_data.end.row {
        let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
        let char_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, yank_data.start);
        let start_idx = buffer.rope().line_to_char(yank_data.start.row);

        // Move yank_data end to end of upper line and make yank_data inclusive.
        // This is the exclusive/inclusive exception.
        let mut end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, yank_data.end);
        if buffer.rope().char(end_idx - 1) == '\n' {
            end_idx = end_idx.saturating_sub(2);
        }
        yank_data.end = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, end_idx);
        yank_data.inclusive = true;

        if buffer
            .rope()
            .slice(start_idx..char_idx)
            .chars()
            .all(|c| c.is_whitespace())
        {
            // exclusive/linewise exception
            yank_data.mode = MotionMode::Linewise;
        }
    }
}

// Promotes eligible characterwise delete ranges spanning multiple lines to
// linewise ranges.
//
// `ctx` provides the active buffer and display configuration, while
// `yank_data` describes and receives the delete range.
fn fix_d(ctx: &mut EditorCtx, yank_data: &mut YankData) {
    if yank_data.start.row < yank_data.end.row {
        let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

        let start_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, yank_data.start);
        let end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, yank_data.end);

        let before_idx = buffer.rope().line_to_char(yank_data.start.row);
        let all_blanks_before = buffer
            .rope()
            .slice(before_idx..start_idx)
            .chars()
            .all(|c| c.is_whitespace());

        if all_blanks_before {
            let after_idx = if yank_data.end.row + 1 >= buffer.rope().len_lines() {
                buffer.rope().len_chars()
            } else {
                buffer.rope().line_to_char(yank_data.end.row + 1)
            };

            let past_end_col =
                next_col_or_display_width(&ctx.config, buffer.rope(), buf_view, yank_data.end);
            let past_end_idx = coords_to_char_idx(
                &ctx.config,
                buffer.rope(),
                buf_view,
                Coords::new(yank_data.end.row, past_end_col),
            );

            if buffer
                .rope()
                .slice(past_end_idx..after_idx)
                .chars()
                .all(|c| c.is_whitespace())
            {
                yank_data.mode = MotionMode::Linewise
            }
        }
    }
}

// --- Yank text based on precomputed YankData ---

fn yank_charwise(ctx: &mut EditorCtx, yank_data: YankData) -> RegisterData {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let rope = buffer.rope();

    let end = if yank_data.inclusive {
        let end_col =
            next_col_or_display_width(&ctx.config, buffer.rope(), buf_view, yank_data.end);
        Coords::new(yank_data.end.row, end_col)
    } else {
        yank_data.end
    };

    let start_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, yank_data.start);
    let end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, end);

    assert!(start_idx <= end_idx);
    let data = rope.slice(start_idx..end_idx).to_string();
    RegisterData::char(data)
}

fn yank_linewise(ctx: &mut EditorCtx, yank_data: YankData) -> RegisterData {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let rope = buffer.rope();

    let start_idx = rope.line_to_char(yank_data.start.row);
    let end_idx = if yank_data.end.row + 1 >= rope.len_lines() {
        rope.len_chars()
    } else {
        rope.line_to_char(yank_data.end.row + 1)
    };

    assert!(start_idx <= end_idx);
    let mut data = rope.slice(start_idx..end_idx).to_string();

    // Line selections always end with a '\n'
    if !data.ends_with('\n') {
        data.push('\n');
    }

    RegisterData::line(data)
}

fn yank_blockwise(ctx: &mut EditorCtx, yank_data: YankData) -> RegisterData {
    let start = yank_data.start;
    let end = yank_data.end;

    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let rope = buffer.rope();

    let tl = Coords::new(start.row, start.col.min(end.col));
    let br = Coords::new(end.row + 1, start.col.max(end.col) + 1);

    let mut rows = Vec::with_capacity(br.row - tl.row);

    for row in tl.row..br.row {
        let line_idx = rope.line_to_char(row);
        let line = display_line(&ctx.config, rope, buf_view, row);
        let mut curr_row = String::with_capacity(br.col - tl.col);

        for (g, span) in line.graphemes_between(tl.col, br.col) {
            if span.start < tl.col && span.end > br.col {
                curr_row.extend(std::iter::repeat_n(' ', br.col - tl.col));
            } else if span.start < tl.col {
                curr_row.extend(std::iter::repeat_n(' ', span.end - tl.col));
            } else if span.end > br.col {
                curr_row.extend(std::iter::repeat_n(' ', br.col - span.start));
            } else {
                let line_idx = buffer.rope().line_to_char(row);
                if rope.char(line_idx + line.col_to_char_idx(span.start)) == '\t' {
                    curr_row.push('\t');
                } else {
                    curr_row.push_str(g);
                }
            }
        }

        rows.push(curr_row);
    }

    RegisterData::block(rows)
}
