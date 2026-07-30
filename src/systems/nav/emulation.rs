use anyhow::{Result, anyhow};
use ropey::Rope;

use crate::{
    active_session, active_session_and_buffer,
    cmd::{Cmd, Motion, MotionMode, Operator},
    components::{Coords, DisplayLine, DisplayLineRef, EditorCtx, RegisterData},
    systems::{
        commons::coords_to_char_idx,
        nav::{NavArgs, handle_session_nav},
    },
};

pub fn emulate_motion(
    ctx: &mut EditorCtx,
    m: Motion,
    cmd_reps: usize,
    arg_reps: usize,
    forced_mode: Option<MotionMode>,
) -> Result<RegisterData> {
    if uses_viewport(m) {
        return Err(anyhow!("Invalid mnotion"));
    }

    let (orig_cursor, orig_target_col) = {
        let (_, buf_view) = active_session!(ctx);
        let cursor = buf_view.cursor;
        let target_col = buf_view.target_col;
        (cursor, target_col)
    };

    let mut incl = inclusive(ctx, m);
    if charwise(m) && !incl {
        incl = eval_exclusive_charwise(ctx, m, cmd_reps, arg_reps);
    } else {
        eval(ctx, m, cmd_reps, arg_reps);
    }

    // TODO
    // Handle exceptions in the vim reference manual (exclusive -> inclusive and exclusive -> linewise)

    let mode = forced_mode.unwrap_or_else(|| {
        if linewise(m) {
            MotionMode::Linewise
        } else {
            MotionMode::Charwise
        }
    });

    if let Some(MotionMode::Charwise) = forced_mode
        && charwise(m)
    {
        incl = !incl;
    }

    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let end_cursor = buf_view.cursor;

    if end_cursor > orig_cursor {
        buf_view.cursor = orig_cursor;
        buf_view.target_col = orig_target_col;
    }

    Ok(match mode {
        MotionMode::Charwise => adjust_charwise(ctx, (orig_cursor, end_cursor), incl),
        MotionMode::Linewise => adjust_linewise(ctx, (orig_cursor, end_cursor)),
        MotionMode::Blockwise => adjust_blockwise(ctx, (orig_cursor, end_cursor)),
    })
}

// Exclusive Charwise movements become inclusive if they "bounce"
// (they try to go beyond the end of line). In order to detect a
// bounce, we need a special evaluator.
//
// Returns true if the move must become inclusive.
fn eval_exclusive_charwise(
    ctx: &mut EditorCtx,
    m: Motion,
    cmd_reps: usize,
    arg_reps: usize,
) -> bool {
    let mut inclusive = false;
    let mut last_col = None;

    for _ in 0..cmd_reps {
        for _ in 0..arg_reps {
            let cmd = Cmd::new(Operator::Move(m)).reps(Some(1));
            let args = NavArgs::new(m, cmd);
            handle_session_nav(ctx, args);

            let (_, buf_view) = active_session!(ctx);
            if let Some(col) = last_col
                && buf_view.cursor.col == col
            {
                inclusive = true;
                break;
            }
            last_col = Some(buf_view.cursor.col);
        }
    }

    inclusive
}

// Eval non-charwise exclusive motions
fn eval(ctx: &mut EditorCtx, m: Motion, cmd_reps: usize, arg_reps: usize) {
    let mut cmd_reps = cmd_reps;
    let mut arg_reps = arg_reps;

    // Line motion (what you get in commands like yy or dd) is sort of a hack.
    // To get proper emulation, you have to execute all the line motions in
    // one sweep, so we move all the intended reps to arg_reps.
    if m == Motion::Line {
        arg_reps *= cmd_reps;
        cmd_reps = 1;
    }

    for _ in 0..cmd_reps {
        let cmd = Cmd::new(Operator::Move(m)).reps(Some(arg_reps));
        let args = NavArgs::new(m, cmd);
        handle_session_nav(ctx, args);
    }
}

fn adjust_charwise(ctx: &mut EditorCtx, span: (Coords, Coords), inclusive: bool) -> RegisterData {
    let (start, mut end) = span;
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let rope = buffer.rope();

    if inclusive {
        let line = buf_view.display_buf.ensure_line(&ctx.config, rope, end.row);
        end.col = go_right(line, end.col);
    }

    let mut start_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, start);
    let mut end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, end);

    if start_idx > end_idx {
        std::mem::swap(&mut start_idx, &mut end_idx);
    }

    // Charwise ranges must not end in '\n'
    if end_idx > 0 && rope.char(end_idx - 1) == '\n' {
        end_idx -= 1;
    }

    let slice = buffer.rope().slice(start_idx..end_idx);
    RegisterData::char(slice.into())
}

fn adjust_linewise(ctx: &mut EditorCtx, span: (Coords, Coords)) -> RegisterData {
    let (start, end) = span;
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let rope = buffer.rope();

    let mut start_idx = coords_to_char_idx(&ctx.config, rope, buf_view, start);
    let mut end_idx = coords_to_char_idx(&ctx.config, rope, buf_view, end);

    if start_idx > end_idx {
        std::mem::swap(&mut start_idx, &mut end_idx);
    }

    let start_line = rope.char_to_line(start_idx);
    let start_line_idx = rope.line_to_char(start_line);

    let end_line = rope.char_to_line(end_idx);
    let end_line_idx = if end_line + 1 == rope.len_lines() {
        rope.len_chars()
    } else {
        rope.line_to_char(end_line + 1)
    };

    let mut rope = Rope::from(rope.slice(start_line_idx..end_line_idx));

    let len = rope.len_chars();
    if len > 0 && rope.char(len - 1) != '\n' {
        rope.insert_char(len, '\n');
    }

    RegisterData::line(rope)
}

fn adjust_blockwise(ctx: &mut EditorCtx, span: (Coords, Coords)) -> RegisterData {
    let (start, end) = span;
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let rope = buffer.rope();

    let tl = Coords::new(start.row.min(end.row), start.col.min(end.col));
    let mut br = Coords::new(start.row.max(end.row), start.col.max(end.col));
    br.col += 1;

    let mut rows = Vec::with_capacity(br.row - tl.row + 1);

    for row in tl.row..=br.row {
        let mut curr_row = Rope::new();
        let line = buf_view.display_buf.ensure_line(&ctx.config, rope, row);
        for (g, span) in line.graphemes_between(tl.col, br.col + 1) {
            if span.start < tl.col && span.end > br.col {
                curr_row.append(" ".repeat(br.col - tl.col).into());
            } else if span.start < tl.col {
                curr_row.append(" ".repeat(span.end - tl.col).into());
            } else if span.end > br.col {
                curr_row.append(" ".repeat(br.col - span.start).into());
            } else {
                if DisplayLine::is_tab(g) {
                    curr_row.insert_char(curr_row.len_chars(), '\t');
                } else {
                    curr_row.append(g.into());
                }
            }
        }
        rows.push(curr_row);
    }

    RegisterData::block(rows)
}

// Expand the given column one grapheme to the right if possible.
fn go_right(line: DisplayLineRef<'_>, col: usize) -> usize {
    let next_col = line.next_col(col);
    if next_col > col {
        next_col
    } else {
        line.display_width
    }
}

// -----------------------------------------------------------------------
// Movement properties
// We do out best to follow what the vim reference manual says.
// See :help motions
// -----------------------------------------------------------------------
fn uses_viewport(m: Motion) -> bool {
    match m {
        Motion::PageDown | Motion::PageUp => true,
        _ => false,
    }
}

fn linewise(m: Motion) -> bool {
    match m {
        Motion::Down => true,
        Motion::Up => true,
        Motion::Line => true,
        Motion::GotoLine(_) => true,
        Motion::FirstNonBlankInFile => true,
        Motion::StartOfFile => true,
        Motion::EndOfFile => true,
        Motion::GotoMark(_) => true,

        _ => false,
    }
}

fn charwise(m: Motion) -> bool {
    !linewise(m)
}

fn inclusive(ctx: &EditorCtx, m: Motion) -> bool {
    match m {
        _ if linewise(m) => true,

        Motion::EndOfLine => true,
        Motion::FindNextChar(_) => true,
        Motion::TillNextChar(_) => true,
        Motion::EndSubWord => true,
        Motion::EndBigWord => true,
        Motion::RepeatBackward => ctx.search.char_search().is_some_and(|m| !inclusive(ctx, m)),
        Motion::RepeatForward => ctx.search.char_search().is_some_and(|m| inclusive(ctx, m)),

        _ => false,
    }
}
