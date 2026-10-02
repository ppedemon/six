use ratatui::layout::Rect;
use ropey::Rope;

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

// --- yank_charwise ---

#[test]
fn yank_charwise_includes_the_end_character_when_inclusive() {
    let mut ctx = setup("abcd", Coords::default());
    let yank_data = YankData::new(
        Coords::new(0, 1),
        Coords::new(0, 2),
        MotionMode::Charwise,
        true,
    );

    assert_eq!(
        yank_charwise(&mut ctx, yank_data),
        RegisterData::char("bc".to_owned())
    );
}

#[test]
fn yank_charwise_excludes_the_end_position_when_exclusive() {
    let mut ctx = setup("abcd", Coords::default());
    let yank_data = YankData::new(
        Coords::new(0, 1),
        Coords::new(0, 2),
        MotionMode::Charwise,
        false,
    );

    assert_eq!(
        yank_charwise(&mut ctx, yank_data),
        RegisterData::char("b".to_owned())
    );
}

#[test]
#[should_panic]
fn yank_charwise_rejects_a_reversed_range() {
    let mut ctx = setup("abcd", Coords::default());
    let yank_data = YankData {
        start: Coords::new(0, 2),
        end: Coords::new(0, 1),
        mode: MotionMode::Charwise,
        inclusive: false,
    };

    yank_charwise(&mut ctx, yank_data);
}

// --- yank_linewise ---

#[test]
fn yank_linewise_ends_at_the_next_line_when_selection_is_not_at_eof() {
    let mut ctx = setup("one\ntwo\nthree", Coords::default());
    let yank_data = YankData::new(
        Coords::default(),
        Coords::default(),
        MotionMode::Linewise,
        true,
    );

    assert_eq!(
        yank_linewise(&mut ctx, yank_data),
        RegisterData::line("one\n".to_owned())
    );
}

#[test]
fn yank_linewise_adds_a_newline_for_a_final_line_without_one() {
    let mut ctx = setup("one\ntwo", Coords::default());
    let yank_data = YankData::new(
        Coords::new(1, 0),
        Coords::new(1, 0),
        MotionMode::Linewise,
        true,
    );

    assert_eq!(
        yank_linewise(&mut ctx, yank_data),
        RegisterData::line("two\n".to_owned())
    );
}

#[test]
fn yank_linewise_preserves_a_newline_on_the_final_line() {
    let mut ctx = setup("one\ntwo\n", Coords::default());
    let yank_data = YankData::new(
        Coords::new(1, 0),
        Coords::new(1, 0),
        MotionMode::Linewise,
        true,
    );

    assert_eq!(
        yank_linewise(&mut ctx, yank_data),
        RegisterData::line("two\n".to_owned())
    );
}

#[test]
#[should_panic]
fn yank_linewise_rejects_a_reversed_range() {
    let mut ctx = setup("one\ntwo", Coords::default());
    let yank_data = YankData {
        start: Coords::new(1, 0),
        end: Coords::default(),
        mode: MotionMode::Linewise,
        inclusive: true,
    };

    yank_linewise(&mut ctx, yank_data);
}

// --- yank_blockwise ---

#[test]
fn yank_blockwise_pads_when_a_grapheme_crosses_both_selection_edges() {
    let mut ctx = setup("\t", Coords::default());
    let yank_data = YankData::new(
        Coords::new(0, 1),
        Coords::new(0, 2),
        MotionMode::Blockwise,
        true,
    );

    assert_eq!(
        yank_blockwise(&mut ctx, yank_data),
        RegisterData::block(vec!["  ".to_owned()])
    );
}

#[test]
fn yank_blockwise_pads_the_left_edge_of_a_grapheme() {
    let mut ctx = setup("\t", Coords::default());
    let yank_data = YankData::new(
        Coords::new(0, 2),
        Coords::new(0, 4),
        MotionMode::Blockwise,
        true,
    );

    assert_eq!(
        yank_blockwise(&mut ctx, yank_data),
        RegisterData::block(vec!["   ".to_owned()])
    );
}

#[test]
fn yank_blockwise_pads_the_right_edge_of_a_grapheme() {
    let mut ctx = setup("\ta", Coords::default());
    let yank_data = YankData::new(
        Coords::new(0, 4),
        Coords::new(0, 8),
        MotionMode::Blockwise,
        true,
    );

    assert_eq!(
        yank_blockwise(&mut ctx, yank_data),
        RegisterData::block(vec!["    a".to_owned()])
    );
}

#[test]
fn yank_blockwise_copies_text_and_pads_a_clipped_tab() {
    let mut ctx = setup("aa\t", Coords::default());
    let yank_data = YankData::new(
        Coords::new(0, 1),
        Coords::new(0, 4),
        MotionMode::Blockwise,
        true,
    );

    assert_eq!(
        yank_blockwise(&mut ctx, yank_data),
        RegisterData::block(vec!["a   ".to_owned()])
    );
}

#[test]
fn yank_blockwise_preserves_a_whole_tab() {
    let mut ctx = setup("\t", Coords::default());
    let yank_data = YankData::new(
        Coords::default(),
        Coords::new(0, 7),
        MotionMode::Blockwise,
        true,
    );

    assert_eq!(
        yank_blockwise(&mut ctx, yank_data),
        RegisterData::block(vec!["\t".to_owned()])
    );
}

#[test]
fn yank_blockwise_copies_a_non_tab_grapheme() {
    let mut ctx = setup("a", Coords::default());
    let yank_data = YankData::new(
        Coords::default(),
        Coords::default(),
        MotionMode::Blockwise,
        true,
    );

    assert_eq!(
        yank_blockwise(&mut ctx, yank_data),
        RegisterData::block(vec!["a".to_owned()])
    );
}

#[test]
fn yank_blockwise_pads_a_partially_selected_emoji() {
    let mut ctx = setup("a😀b", Coords::default());
    let yank_data = YankData::new(
        Coords::new(0, 2),
        Coords::new(0, 3),
        MotionMode::Blockwise,
        true,
    );

    assert_eq!(
        yank_blockwise(&mut ctx, yank_data),
        RegisterData::block(vec![" b".to_owned()])
    );
}
