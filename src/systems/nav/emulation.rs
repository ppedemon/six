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
    let mut forward = false;
    let mut inclusive = false;
    let mut last_coord = {
        let (_, buf_view) = active_session!(ctx);
        buf_view.cursor
    };

    for _ in 0..cmd_reps {
        for _ in 0..arg_reps {
            let cmd = Cmd::new(Operator::Move(m)).reps(Some(1));
            let args = NavArgs::new(m, cmd);
            handle_session_nav(ctx, args);

            let (_, buf_view) = active_session!(ctx);
            if buf_view.cursor == last_coord {
                inclusive = true;
                break;
            }
            forward = forward || (buf_view.cursor > last_coord);
            last_coord = buf_view.cursor;
        }
    }

    inclusive && forward
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
    let (mut start, mut end) = span;
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let rope = buffer.rope();

    if start > end {
        std::mem::swap(&mut start, &mut end);
    }

    if inclusive {
        let line = buf_view.display_buf.ensure_line(&ctx.config, rope, end.row);
        end.col = go_right(line, end.col);
    }

    let start_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, start);
    let mut end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, end);

    // Charwise ranges must not end in '\n'
    if end_idx > 0 && rope.char(end_idx - 1) == '\n' {
        end_idx -= 1;
    }

    let selection = safe_slice(rope, start_idx, end_idx);
    RegisterData::char(selection)
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

    let mut rope = safe_slice(rope, start_line_idx, end_line_idx);

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

        if line.display_width > 0 {
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

fn safe_slice(rope: &Rope, start_idx: usize, end_idx: usize) -> Rope {
    assert!(start_idx <= end_idx);
    if start_idx == end_idx {
        Rope::new()
    } else {
        rope.slice(start_idx..end_idx).into()
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

#[cfg(test)]
mod test {
    use ratatui::layout::Rect;
    use std::assert_eq;

    use super::*;
    use crate::components::{Buffer, BufferView, Session, Viewport};

    // Test setup: editor with given text and cursor at given coords
    fn setup(text: &str, cursor: Coords) -> EditorCtx {
        let mut ctx = EditorCtx::new();

        let rope: Rope = text.into();
        let buffer = Buffer::new(rope);
        let buf_id = ctx.spawn_buffer(buffer);

        let mut buf_view = BufferView::empty();
        buf_view.cursor = cursor;

        let mut session = Session::empty(buf_id);
        session.viewport = Viewport {
            scroll: Coords::default(),
            area: Rect::new(0, 0, 80, 24),
        };

        let session_id = ctx.spawn_session(session, buf_view);
        ctx.editor.session_id = session_id;
        ctx
    }

    #[test]
    fn test_reject_viewport_motions() {
        let mut ctx = setup("", Coords::default());
        let m = Motion::PageUp;
        let result = emulate_motion(&mut ctx, m, 1, 1, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_rope() {
        let mut ctx = setup("", Coords::default());
        let m = Motion::Right;
        let result = emulate_motion(&mut ctx, m, 10, 10, None);
        assert_eq!(result.unwrap(), RegisterData::char("".into()));
    }

    #[test]
    fn test_moot_movement() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::StartOfLine;
        let result = emulate_motion(&mut ctx, m, 1, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("".into()));
    }

    #[test]
    fn test_inclusive_charwise() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::EndSubWord;
        let result = emulate_motion(&mut ctx, m, 1, 2, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo bar".into()));
    }

    #[test]
    fn test_exclusive_charwise() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::Right;
        let result = emulate_motion(&mut ctx, m, 6, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo ba".into()));
    }

    #[test]
    fn test_exclusive_charwise_bounce() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::Right;
        let result = emulate_motion(&mut ctx, m, 7, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo bar".into()));
    }

    #[test]
    fn test_inclusive_charwise_wide() {
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let m = Motion::EndSubWord;
        let result = emulate_motion(&mut ctx, m, 1, 2, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo\t🧑‍🧑‍🧒‍🧒".into()));
    }

    #[test]
    fn test_exclusive_charwise_toggle() {
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let m = Motion::Right;

        let result = emulate_motion(&mut ctx, m, 3, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo".into()));

        let result = emulate_motion(&mut ctx, m, 4, 1, Some(MotionMode::Charwise));
        assert_eq!(result.unwrap(), RegisterData::char("foo\t🧑‍🧑‍🧒‍🧒".into()));
    }

    #[test]
    fn test_charwise_backwards() {
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::new(0, 8));
        let m = Motion::Left;
        let result = emulate_motion(&mut ctx, m, 1, 4, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo\t".into()));
    }

    #[test]
    fn test_charwise_backwards_bounce() {
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::new(0, 8));
        let m = Motion::Left;
        let result = emulate_motion(&mut ctx, m, 1, 1000, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo\t".into()));
    }

    #[test]
    fn test_exclusive_charwise_bounce_toggle() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::Right;
        let result = emulate_motion(&mut ctx, m, 7, 1, Some(MotionMode::Charwise));
        assert_eq!(result.unwrap(), RegisterData::char("foo ba".into()));
    }

    #[test]
    fn test_charwise_backwards_bounce_toggle() {
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::new(0, 8));
        let m = Motion::Left;
        let result = emulate_motion(&mut ctx, m, 1, 1000, Some(MotionMode::Charwise));
        assert_eq!(result.unwrap(), RegisterData::char("foo\t🧑‍🧑‍🧒‍🧒".into()));
    }

    #[test]
    fn test_vi_difference() {
        let mut ctx = setup("f", Coords::new(0, 0));

        // In vi a moot bounding backwards motion is exclusive, we do the same
        let result = emulate_motion(&mut ctx, Motion::Left, 1, 1000, None);
        assert_eq!(result.unwrap(), RegisterData::char("".into()));

        // But a moot bounding forward motion is *inclusive*, vi returns 'f'.
        // But we can't detect forward/backards motion if the cursor doesn't move - we still return an empty selection
        // I guess vi implemements forward movements by overshooting, and marking the overshoot somewhere.
        // I say f*ck it. This is an edge case.
        let result = emulate_motion(&mut ctx, Motion::Right, 1, 1000, None);
        assert_eq!(result.unwrap(), RegisterData::char("".into()));
    }
}
