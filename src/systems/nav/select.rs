use ropey::Rope;

use crate::{
    active_session, active_session_and_buffer,
    cmd::{Cmd, Motion, Operator},
    components::{Coords, DisplayLineRef, EditorCtx, RegisterData},
    systems::{
        commons::coords_to_char_idx,
        nav::{NavArgs, handle_session_nav},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionExtent {
    pub start: Coords,
    pub end: Coords,
    pub overshot: bool,
}

impl MotionExtent {
    pub fn new(start: Coords, end: Coords, overshot: bool) -> Self {
        Self {
            start,
            end,
            overshot,
        }
    }
}

pub fn exec_motion(
    ctx: &mut EditorCtx,
    m: Motion,
    cmd_reps: usize,
    arg_reps: usize,
) -> Option<MotionExtent> {
    if uses_viewport(m) {
        return None;
    }

    let (_, buf_view) = active_session!(ctx);
    let start = buf_view.cursor;

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

    let (_, buf_view) = active_session!(ctx);
    let end = buf_view.cursor;

    Some(MotionExtent {
        start,
        end,
        overshot: buf_view.overshot,
    })
}

// Do a charwise selection for the given motion extent
pub fn select_charwise(
    ctx: &mut EditorCtx,
    span: (Coords, Coords),
    inclusive: bool,
) -> RegisterData {
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

pub fn select_linewise(ctx: &mut EditorCtx, span: (Coords, Coords)) -> RegisterData {
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

    let mut data = safe_slice(rope, start_line_idx, end_line_idx);
    if !data.ends_with('\n') {
        data.push('\n');
    }

    RegisterData::line(data)
}

pub fn select_blockwise(ctx: &mut EditorCtx, span: (Coords, Coords)) -> RegisterData {
    let (start, end) = span;
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let rope = buffer.rope();

    let tl = Coords::new(start.row.min(end.row), start.col.min(end.col));
    let br = Coords::new(start.row.max(end.row) + 1, start.col.max(end.col) + 1);

    let mut rows = Vec::with_capacity(br.row - tl.row);

    for row in tl.row..br.row {
        let mut curr_row = String::new();
        let line = buf_view.display_buf.ensure_line(&ctx.config, rope, row);

        for (g, span) in line.graphemes_between(tl.col, br.col) {
            if span.start < tl.col && span.end > br.col {
                curr_row.extend(std::iter::repeat(' ').take(br.col - tl.col));
            } else if span.start < tl.col {
                curr_row.extend(std::iter::repeat(' ').take(span.end - tl.col));
            } else if span.end > br.col {
                curr_row.extend(std::iter::repeat(' ').take(br.col - span.start));
            } else {
                if rope.char(line.col_to_char_idx(span.start)) == '\t' {
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

// Move the given column one grapheme to the right if possible.
fn go_right(line: DisplayLineRef<'_>, col: usize) -> usize {
    let next_col = line.next_col(col);
    if next_col > col {
        next_col
    } else {
        line.display_width
    }
}

fn safe_slice(rope: &Rope, start_idx: usize, end_idx: usize) -> String {
    assert!(start_idx <= end_idx);
    if start_idx == end_idx {
        String::new()
    } else {
        rope.slice(start_idx..end_idx).to_string()
    }
}

// -----------------------------------------------------------------------
// Movement properties
// We do out best to follow what the vim reference manual says.
// See (in vim) :help motions
// -----------------------------------------------------------------------
fn uses_viewport(m: Motion) -> bool {
    match m {
        Motion::PageDown | Motion::PageUp => true,
        _ => false,
    }
}

pub fn linewise(m: Motion) -> bool {
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

pub fn charwise(m: Motion) -> bool {
    !linewise(m)
}

pub fn inclusive(ctx: &EditorCtx, m: Motion) -> bool {
    match m {
        _ if linewise(m) => true,

        Motion::EndOfLine => true,
        Motion::FindNextChar(_) => true,
        Motion::TillNextChar(_) => true,
        Motion::EndSubWord => true,
        Motion::EndBigWord => true,
        Motion::RepeatBackward => ctx
            .last_search
            .last_char_search()
            .is_some_and(|m| !inclusive(ctx, m)),
        Motion::RepeatForward => ctx
            .last_search
            .last_char_search()
            .is_some_and(|m| inclusive(ctx, m)),

        _ => false,
    }
}

#[cfg(test)]
mod test_commons {
    use super::*;
    use crate::components::{Buffer, BufferView, Session, Viewport};

    use ratatui::layout::Rect;
    use ropey::Rope;

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
mod test_exec {
    use super::test_commons::setup;
    use super::*;

    #[test]
    fn test_err() {
        let mut ctx = setup("", Coords::default());
        let m = Motion::PageUp;
        let result = exec_motion(&mut ctx, m, 1, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_empty() {
        let mut ctx = setup("", Coords::default());
        let m = Motion::Right;
        let result = exec_motion(&mut ctx, m, 10, 10);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::default(), true)
        );
    }

    #[test]
    fn test_moot_movement_bwd() {
        // Backwards is exclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::StartOfLine;
        let result = exec_motion(&mut ctx, m, 1, 1);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::default(), false)
        );
    }

    #[test]
    fn test_moot_movement_fwd() {
        // Forward is inclusive
        let mut ctx = setup("foo bar\nbaz baz", Coords::new(0, 6));
        let m = Motion::EndOfLine;
        let result = exec_motion(&mut ctx, m, 1, 1);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::new(0, 6), Coords::new(0, 6), false)
        );
    }

    #[test]
    fn test_vi_compliance() {
        let mut ctx = setup("f", Coords::new(0, 0));

        // In vi a moot bounding backwards motion is exclusive
        let result = exec_motion(&mut ctx, Motion::Left, 1, 1000);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::default(), false)
        );

        // But a moot bounding forward motion is *inclusive*
        let result = exec_motion(&mut ctx, Motion::Right, 1, 1000);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::default(), true)
        );
    }

    #[test]
    fn test_overshots() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::EndSubWord;
        let result = exec_motion(&mut ctx, m, 1, 4);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::new(1, 6), false)
        );

        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::EndSubWord;
        let result = exec_motion(&mut ctx, m, 1, 5);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::new(1, 6), true)
        );
    }

    #[test]
    fn test_fwd() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::Right;
        let result = exec_motion(&mut ctx, m, 6, 1);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::new(0, 6), false)
        );
    }

    #[test]
    fn test_bwd() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::new(0, 6));
        let m = Motion::PrevSubWord;
        let result = exec_motion(&mut ctx, m, 1, 1);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::new(0, 6), Coords::new(0, 4), false)
        );

        // Backwards motions never overshoot
        let mut ctx = setup("foo bar\nbaz baz", Coords::new(0, 6));
        let m = Motion::PrevSubWord;
        let result = exec_motion(&mut ctx, m, 1000, 1);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::new(0, 6), Coords::default(), false)
        );
    }

    #[test]
    fn test_funny_chars() {
        // Forward, no overshot
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let m = Motion::EndSubWord;
        let result = exec_motion(&mut ctx, m, 1, 2);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::new(0, 8), false)
        );

        // Forward, overshot
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let m = Motion::EndSubWord;
        let result = exec_motion(&mut ctx, m, 1, 5);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::new(1, 6), true)
        );

        // Backwards
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::new(1, 0));
        let m = Motion::PrevSubWord;
        let result = exec_motion(&mut ctx, m, 1, 1);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::new(1, 0), Coords::new(0, 8), false)
        );
    }

    #[test]
    fn test_multiline_fwd() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let m = Motion::GotoLine(2);
        let result = exec_motion(&mut ctx, m, 3, 1);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::default(), Coords::new(1, 0), false)
        );
    }

    #[test]
    fn test_multiline_bwd() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::new(1, 4));
        let m = Motion::PrevSubWord;
        let result = exec_motion(&mut ctx, m, 2, 1);
        assert_eq!(
            result.unwrap(),
            MotionExtent::new(Coords::new(1, 4), Coords::new(0, 4), false)
        );
    }
}

#[cfg(test)]
mod test_select_charwise {
    use super::test_commons::setup;
    use super::*;

    #[test]
    fn test_empty() {
        let mut ctx = setup("", Coords::default());
        let span = (Coords::default(), Coords::default());
        let data = select_charwise(&mut ctx, span, true);
        assert_eq!(data, RegisterData::char("".into()));
    }

    #[test]
    fn test_fwd() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let span = (Coords::default(), Coords::new(0, 6));
        let data = select_charwise(&mut ctx, span, false);
        assert_eq!(data, RegisterData::char("foo ba".into()));

        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let span = (Coords::default(), Coords::new(0, 6));
        let data = select_charwise(&mut ctx, span, true);
        assert_eq!(data, RegisterData::char("foo bar".into()));
    }

    #[test]
    fn test_bwd() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let span = (Coords::new(0, 4), Coords::default());
        let data = select_charwise(&mut ctx, span, false);
        assert_eq!(data, RegisterData::char("foo ".into()));

        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let span = (Coords::new(0, 4), Coords::default());
        let data = select_charwise(&mut ctx, span, true);
        assert_eq!(data, RegisterData::char("foo b".into()));
    }

    #[test]
    fn test_funny_chars() {
        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let span = (Coords::default(), Coords::new(0, 7));
        let data = select_charwise(&mut ctx, span, false);
        assert_eq!(data, RegisterData::char("foo".into()));

        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let span = (Coords::default(), Coords::new(0, 7));
        let data = select_charwise(&mut ctx, span, true);
        assert_eq!(data, RegisterData::char("foo\t".into()));

        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let span = (Coords::default(), Coords::new(0, 8));
        let data = select_charwise(&mut ctx, span, false);
        assert_eq!(data, RegisterData::char("foo\t".into()));

        let mut ctx = setup("foo\t🧑‍🧑‍🧒‍🧒\nbaz baz", Coords::default());
        let span = (Coords::default(), Coords::new(0, 8));
        let data = select_charwise(&mut ctx, span, true);
        assert_eq!(data, RegisterData::char("foo\t🧑‍🧑‍🧒‍🧒".into()));
    }

    #[test]
    fn test_nl() {
        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let span = (Coords::default(), Coords::new(1, 2));
        let data = select_charwise(&mut ctx, span, false);
        assert_eq!(data, RegisterData::char("foo bar\nba".into()));

        let mut ctx = setup("foo bar\nbaz baz", Coords::default());
        let span = (Coords::default(), Coords::new(0, 7));
        let data = select_charwise(&mut ctx, span, true);
        assert_eq!(data, RegisterData::char("foo bar".into()));
    }
}

#[cfg(test)]
mod test_select_linewise {
    use super::test_commons::setup;
    use super::*;

    #[test]
    fn test_empty() {
        let mut ctx = setup("", Coords::default());
        let span = (Coords::default(), Coords::default());
        let data = select_linewise(&mut ctx, span);
        assert_eq!(data, RegisterData::line("\n".into()));
    }

    #[test]
    fn test_single_line() {
        let mut ctx = setup("foo bar\nbaz baz\nword1 word2", Coords::default());
        let span = (Coords::default(), Coords::default());
        let data = select_linewise(&mut ctx, span);
        assert_eq!(data, RegisterData::line("foo bar\n".into()));
    }

    #[test]
    fn test_empty_line() {
        let mut ctx = setup("foo bar\n\nword1 word2", Coords::default());
        let span = (Coords::default(), Coords::new(1, 0));
        let data = select_linewise(&mut ctx, span);
        assert_eq!(data, RegisterData::line("foo bar\n\n".into()));
    }

    #[test]
    fn test_multi_line() {
        let mut ctx = setup("foo bar\nbaz baz\nword1 word2", Coords::default());
        let span = (Coords::new(0, 3), Coords::new(1, 3));
        let data = select_linewise(&mut ctx, span);
        assert_eq!(data, RegisterData::line("foo bar\nbaz baz\n".into()));
    }

    #[test]
    fn test_backwards() {
        let mut ctx = setup("foo bar\nbaz baz\nword1 word2", Coords::default());
        let span = (Coords::new(1, 3), Coords::new(0, 3));
        let data = select_linewise(&mut ctx, span);
        assert_eq!(data, RegisterData::line("foo bar\nbaz baz\n".into()));
    }

    #[test]
    fn test_whole() {
        let mut ctx = setup("foo bar\nbaz baz\nword1 word2", Coords::default());
        let span = (Coords::default(), Coords::new(2, 3));
        let data = select_linewise(&mut ctx, span);
        assert_eq!(
            data,
            RegisterData::line("foo bar\nbaz baz\nword1 word2\n".into())
        );
    }
}

#[cfg(test)]
mod test_select_blockwise {
    use super::test_commons::setup;
    use super::*;

    #[test]
    fn test_empty() {
        let mut ctx = setup("", Coords::default());
        let span = (Coords::default(), Coords::default());
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(data, RegisterData::block(Vec::new()));
    }

    #[test]
    fn test_singles() {
        // Single char
        let mut ctx = setup("foo bar\nbaz baz\nword1 word2", Coords::default());
        let span = (Coords::new(0, 4), Coords::new(0, 4));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(data, RegisterData::block(vec!["b".into()]));

        // Single line
        let mut ctx = setup("foo bar\nbaz baz\nword1 word2", Coords::default());
        let span = (Coords::default(), Coords::new(0, 2));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(data, RegisterData::block(vec!["foo".into()]));

        // Single column
        let mut ctx = setup("foo bar\nbaz baz\nword1 word2", Coords::default());
        let span = (Coords::new(0, 4), Coords::new(2, 4));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(
            data,
            RegisterData::block(vec!["b".into(), "b".into(), "1".into()])
        );
    }

    #[test]
    fn test_diagonals() {
        // TL → BR
        let mut ctx = setup("aaaaaa\nbbbbbb", Coords::default());
        let span = (Coords::new(0, 2), Coords::new(1, 3));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(data, RegisterData::block(vec!["aa".into(), "bb".into()]));

        // TR → BL
        let mut ctx = setup("aaaaaa\nbbbbbb", Coords::default());
        let span = (Coords::new(0, 3), Coords::new(1, 2));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(data, RegisterData::block(vec!["aa".into(), "bb".into()]));

        // BR → TL
        let mut ctx = setup("aaaaaa\nbbbbbb", Coords::default());
        let span = (Coords::new(1, 3), Coords::new(0, 2));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(data, RegisterData::block(vec!["aa".into(), "bb".into()]));

        // BL → TR
        let mut ctx = setup("aaaaaa\nbbbbbb", Coords::default());
        let span = (Coords::new(1, 2), Coords::new(0, 3));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(data, RegisterData::block(vec!["aa".into(), "bb".into()]));
    }

    #[test]
    fn test_empty_line() {
        let mut ctx = setup("foo bar\n\nword1 word2", Coords::default());
        let span = (Coords::new(0, 4), Coords::new(2, 6));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(
            data,
            RegisterData::block(vec!["bar".into(), "".into(), "1 w".into()])
        );
    }

    #[test]
    fn test_overflow_both() {
        let mut ctx = setup("aaaaaaaaa\n|\t|\naaaaaaaaa", Coords::default());
        let span = (Coords::new(0, 2), Coords::new(2, 5));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(
            data,
            RegisterData::block(vec!["aaaa".into(), "    ".into(), "aaaa".into()])
        );
    }

    #[test]
    fn test_overflow_left() {
        let mut ctx = setup("aaaaaaaaa\n|\t|\naaaaaaaaa", Coords::default());
        let span = (Coords::new(0, 4), Coords::new(2, 8));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(
            data,
            RegisterData::block(vec!["aaaaa".into(), "    |".into(), "aaaaa".into()])
        );
    }

    #[test]
    fn test_overflow_right() {
        let mut ctx = setup("aaaaaaaaa\n|\t|\naaaaaaaaa", Coords::default());
        let span = (Coords::new(0, 0), Coords::new(2, 3));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(
            data,
            RegisterData::block(vec!["aaaa".into(), "|   ".into(), "aaaa".into()])
        );
    }

    #[test]
    fn test_perfect() {
        let mut ctx = setup("aaa bbb ccc\naaa bbb ccc\naaa bbb ccc", Coords::default());
        let span = (Coords::new(0, 4), Coords::new(2, 6));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(
            data,
            RegisterData::block(vec!["bbb".into(), "bbb".into(), "bbb".into()])
        );
    }

    #[test]
    fn test_weird() {
        let mut ctx = setup("foo\tbar\nf🏳️‍🌈 bar", Coords::default());
        let span = (Coords::new(0, 2), Coords::new(1, 6));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(
            data,
            RegisterData::block(vec!["o    ".into(), "  bar".into()])
        );
    }

    #[test]
    fn test_jagged() {
        let mut ctx = setup("foo\tbar\n\naaa\nf🏳️‍🌈 bar", Coords::default());
        let span = (Coords::new(0, 2), Coords::new(3, 6));
        let data = select_blockwise(&mut ctx, span);
        assert_eq!(
            data,
            RegisterData::block(vec!["o    ".into(), "".into(), "a".into(), "  bar".into()])
        );
    }
}
