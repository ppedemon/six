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
        return Err(anyhow!("Invalid motion"));
    }

    let (orig_cursor, orig_target_col) = {
        let (_, buf_view) = active_session!(ctx);
        let cursor = buf_view.cursor;
        let target_col = buf_view.target_col;
        (cursor, target_col)
    };

    eval(ctx, m, cmd_reps, arg_reps);

    // TODO Handle exceptions in the vim reference manual:
    // exclusive -> inclusive and exclusive -> linewise

    let mode = forced_mode.unwrap_or_else(|| {
        if linewise(m) {
            MotionMode::Linewise
        } else {
            MotionMode::Charwise
        }
    });

    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let end_cursor = buf_view.cursor;

    if end_cursor > orig_cursor {
        buf_view.cursor = orig_cursor;
        buf_view.target_col = orig_target_col;
    }

    Ok(match mode {
        MotionMode::Charwise => adjust_charwise(ctx, (orig_cursor, end_cursor), m, forced_mode),
        MotionMode::Linewise => adjust_linewise(ctx, (orig_cursor, end_cursor)),
        MotionMode::Blockwise => adjust_blockwise(ctx, (orig_cursor, end_cursor)),
    })
}

// Eval motion
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

fn adjust_charwise(
    ctx: &mut EditorCtx,
    span: (Coords, Coords),
    m: Motion,
    forced_mode: Option<MotionMode>,
) -> RegisterData {
    let mut inclusive = inclusive(ctx, m) || ctx.last_nav.last_nav_overshot();
    if forced_mode.is_some_and(|mode| mode == MotionMode::Charwise) {
        inclusive = !inclusive;
    }

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
        Motion::RepeatBackward => ctx
            .last_nav
            .last_char_search()
            .is_some_and(|m| !inclusive(ctx, m)),
        Motion::RepeatForward => ctx
            .last_nav
            .last_char_search()
            .is_some_and(|m| inclusive(ctx, m)),

        _ => false,
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------
#[cfg(test)]
mod test_commons {
    use super::*;
    use crate::components::{Buffer, BufferView, Session, Viewport};
    use ratatui::layout::Rect;

    // Test setup: editor with given text and cursor at given coords
    pub(crate) fn setup(text: &str, cursor: Coords) -> EditorCtx {
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
}

#[cfg(test)]
mod test_charwise {
    use super::test_commons::setup;
    use super::*;
    use std::assert_eq;

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
    fn test_moot_movement_bwd() {
        // Backwards is exclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::StartOfLine;
        let result = emulate_motion(&mut ctx, m, 1, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("".into()));
    }

    #[test]
    fn test_moot_movement_fwd() {
        // Forward is inclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::new(0, 6));
        let m = Motion::EndOfLine;
        let result = emulate_motion(&mut ctx, m, 1, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("r".into()));
    }

    #[test]
    fn test_vi_compliance() {
        let mut ctx = setup("f", Coords::new(0, 0));

        // In vi a moot bounding backwards motion is exclusive
        let result = emulate_motion(&mut ctx, Motion::Left, 1, 1000, None);
        assert_eq!(result.unwrap(), RegisterData::char("".into()));

        // But a moot bounding forward motion is *inclusive*
        let result = emulate_motion(&mut ctx, Motion::Right, 1, 1000, None);
        assert_eq!(result.unwrap(), RegisterData::char("f".into()));
    }

    #[test]
    fn test_inclusive_fwd() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::EndSubWord;
        let result = emulate_motion(&mut ctx, m, 1, 2, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo bar".into()));
    }

    #[test]
    fn test_exclusive_fwd() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::Right;
        let result = emulate_motion(&mut ctx, m, 6, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo ba".into()));
    }

    #[test]
    fn test_bwd() {
        // All backawrds movements are exclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::new(0, 6));
        let m = Motion::PrevSubWord;
        let result = emulate_motion(&mut ctx, m, 1, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("ba".into()));

        // Backwards movements move the cursor
        let (_, buf_view) = active_session!(ctx);
        assert_eq!(buf_view.cursor, Coords::new(0, 4));
    }

    #[test]
    fn test_funny_chars() {
        // Forward
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let m = Motion::EndSubWord;
        let result = emulate_motion(&mut ctx, m, 1, 2, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo\t🧑‍🧑‍🧒‍🧒".into()));

        // Backwards
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::new(0, 8));
        let m = Motion::PrevSubWord;
        let result = emulate_motion(&mut ctx, m, 1, 2, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo\t".into()));
    }

    #[test]
    fn test_toggle() {
        // Forward
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let m = Motion::Right;
        let result = emulate_motion(&mut ctx, m, 3, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo".into()));

        let result = emulate_motion(&mut ctx, m, 3, 1, Some(MotionMode::Charwise));
        assert_eq!(result.unwrap(), RegisterData::char("foo\t".into()));

        // Toggle forward overshoot from inclusive (since it's an overshoot) to exclusive
        let result = emulate_motion(&mut ctx, m, 20, 1, Some(MotionMode::Charwise));
        assert_eq!(result.unwrap(), RegisterData::char("foo\t".into()));

        // Backward
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::new(0, 8));
        let m = Motion::Left;
        let result = emulate_motion(&mut ctx, m, 4, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo\t".into()));

        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::new(0, 8));
        let m = Motion::Left;
        let result = emulate_motion(&mut ctx, m, 4, 1, Some(MotionMode::Charwise));
        assert_eq!(result.unwrap(), RegisterData::char("foo\t🧑‍🧑‍🧒‍🧒".into()));
    }

    #[test]
    fn test_multiline_fwd() {
        // Forward exclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::NextSubWord;
        let result = emulate_motion(&mut ctx, m, 3, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo bar\nbaz ".into()));

        // Forward inclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::NextSubWord;
        let result = emulate_motion(&mut ctx, m, 20, 1, None);
        assert_eq!(
            result.unwrap(),
            RegisterData::char("foo bar\nbaz baz".into())
        );
    }

    #[test]
    fn test_multiline_bwd() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::new(1, 4));
        let m = Motion::PrevSubWord;
        let result = emulate_motion(&mut ctx, m, 3, 1, None);
        assert_eq!(result.unwrap(), RegisterData::char("foo bar\nbaz ".into()));
    }

    #[test]
    fn test_multiline_toggles() {
        // Forward exclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::NextSubWord;
        let result = emulate_motion(&mut ctx, m, 3, 1, Some(MotionMode::Charwise));
        assert_eq!(result.unwrap(), RegisterData::char("foo bar\nbaz b".into()));

        // Forward inclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::NextSubWord;
        let result = emulate_motion(&mut ctx, m, 20, 1, Some(MotionMode::Charwise));
        assert_eq!(
            result.unwrap(),
            RegisterData::char("foo bar\nbaz ba".into())
        );

        // Make backwards inclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::new(1, 4));
        let m = Motion::PrevSubWord;
        let result = emulate_motion(&mut ctx, m, 3, 1, Some(MotionMode::Charwise));
        assert_eq!(result.unwrap(), RegisterData::char("foo bar\nbaz b".into()));
    }

    #[test]
    fn test_line_coercion() {
        // Single line
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::Left;
        let result = emulate_motion(&mut ctx, m, 1, 1, Some(MotionMode::Linewise));
        assert_eq!(result.unwrap(), RegisterData::line("foo bar\n".into()));

        // Single line
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::GotoLine(2);
        let result = emulate_motion(&mut ctx, m, 1, 1, Some(MotionMode::Linewise));
        assert_eq!(
            result.unwrap(),
            RegisterData::line("foo bar\nbaz baz\n".into())
        );
    }

    #[test]
    fn test_block_coercion() {
        // Perfect block
        let mut ctx = setup("foo bar baz\nfoo bar baz", Coords::new(0, 4));
        let m = Motion::EndSubWord;
        let result = emulate_motion(&mut ctx, m, 1, 4, Some(MotionMode::Blockwise));
        assert_eq!(
            result.unwrap(),
            RegisterData::block(vec!["bar\n".into(), "bar".into()])
        );

        // Weird block
        let mut ctx = setup("foo\tbar\nf🏳️‍🌈 bar", Coords::new(0, 2));
        let m = Motion::EndOfFile;
        let result = emulate_motion(&mut ctx, m, 1, 1, Some(MotionMode::Blockwise));
        assert_eq!(
            result.unwrap(),
            RegisterData::block(vec!["o    \n".into(), "  bar".into()])
        );

        // Jagged block
        let mut ctx = setup("foo\tbar\n\naaa\nf🏳️‍🌈 bar", Coords::new(0, 2));
        let m = Motion::EndOfFile;
        let result = emulate_motion(&mut ctx, m, 1, 1, Some(MotionMode::Blockwise));
        assert_eq!(
            result.unwrap(),
            RegisterData::block(vec![
                "o    \n".into(),
                "".into(),
                "a".into(),
                "  bar".into()
            ])
        );
    }
}
