use std::assert_eq;

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

// --- interpret_motion ---

#[test]
fn interpret_motion_yanks_forward_charwise_range_and_resets_cursor_to_start() {
    let mut ctx = setup("abc", Coords::default());

    let (yank_data, reg_data) = interpret_motion(&mut ctx, Operator::Nop, Motion::Right, None, 1);

    assert_eq!(yank_data.mode, MotionMode::Charwise);
    assert_eq!(yank_data.start, Coords::new(0, 0));
    assert_eq!(yank_data.end, Coords::new(0, 1));
    assert!(!yank_data.inclusive);
    assert_eq!(reg_data, RegisterData::char("a".to_owned()));
    let (_, buf_view) = active_session!(ctx);
    assert_eq!(buf_view.cursor, Coords::new(0, 0));
}

#[test]
fn interpret_motion_normalizes_a_reversed_extent() {
    let mut ctx = setup("abc", Coords::new(0, 2));

    let (yank_data, reg_data) = interpret_motion(&mut ctx, Operator::Nop, Motion::Left, None, 1);

    assert_eq!(yank_data.start, Coords::new(0, 1));
    assert_eq!(yank_data.end, Coords::new(0, 2));
    assert_eq!(reg_data, RegisterData::char("b".to_owned()));
    assert_eq!(yank_data.inclusive, false);
    let (_, buf_view) = active_session!(ctx);
    // The swap path leaves the cursor at the motion's destination.
    assert_eq!(buf_view.cursor, Coords::new(0, 1));
}

#[test]
fn interpret_motion_charwise_override_inverts_natural_inclusivity() {
    let mut ctx = setup("abc", Coords::default());

    let (yank_data, reg_data) = interpret_motion(
        &mut ctx,
        Operator::Nop,
        Motion::EndOfLine,
        Some(MotionMode::Charwise),
        1,
    );

    assert_eq!(yank_data.mode, MotionMode::Charwise);
    assert!(!yank_data.inclusive);
    assert_eq!(reg_data, RegisterData::char("ab".to_owned()));
}

#[test]
fn interpret_motion_linewise_override_is_inclusive() {
    let mut ctx = setup("abc", Coords::default());

    let (yank_data, reg_data) = interpret_motion(
        &mut ctx,
        Operator::Nop,
        Motion::Right,
        Some(MotionMode::Linewise),
        1,
    );

    assert_eq!(yank_data.mode, MotionMode::Linewise);
    assert!(yank_data.inclusive);
    assert_eq!(reg_data, RegisterData::line("abc\n".to_owned()));
}

#[test]
fn interpret_motion_blockwise_override_is_inclusive() {
    let mut ctx = setup("abc", Coords::default());

    let (yank_data, reg_data) = interpret_motion(
        &mut ctx,
        Operator::Nop,
        Motion::Right,
        Some(MotionMode::Blockwise),
        1,
    );

    assert_eq!(yank_data.mode, MotionMode::Blockwise);
    assert!(yank_data.inclusive);
    assert_eq!(reg_data, RegisterData::block(vec!["ab".to_owned()]));
}

#[test]
fn interpret_motion_uses_natural_linewise_mode() {
    let mut ctx = setup("abc", Coords::default());

    let (yank_data, reg_data) = interpret_motion(&mut ctx, Operator::Nop, Motion::Line, None, 1);

    assert_eq!(yank_data.mode, MotionMode::Linewise);
    assert!(yank_data.inclusive);
    assert_eq!(reg_data, RegisterData::line("abc\n".to_owned()));
}

#[test]
fn interpret_motion_runs_word_fix_for_nonempty_text() {
    let mut ctx = setup("one two", Coords::default());

    let (yank_data, reg_data) =
        interpret_motion(&mut ctx, Operator::Nop, Motion::NextBigWord, None, 1);

    assert_eq!(yank_data.mode, MotionMode::Charwise);
    assert_eq!(reg_data, RegisterData::char("one ".to_owned()));
}

#[test]
fn interpret_motion_skips_word_fix_for_empty_text() {
    let mut ctx = setup("", Coords::default());

    let (yank_data, reg_data) =
        interpret_motion(&mut ctx, Operator::Nop, Motion::NextBigWord, None, 1);

    assert_eq!(yank_data.mode, MotionMode::Charwise);
    assert!(reg_data.is_empty());
}

#[test]
fn interpret_motion_uses_overshot_word_motion_as_inclusive() {
    let mut ctx = setup("word", Coords::default());

    let (yank_data, reg_data) =
        interpret_motion(&mut ctx, Operator::Nop, Motion::NextBigWord, None, 1);

    assert!(yank_data.inclusive);
    assert_eq!(reg_data, RegisterData::char("word".to_owned()));
}

#[test]
fn interpret_motion_uses_overshot_word_in_next_line() {
    let mut ctx = setup("word1 word2\nword3", Coords::default());

    let (yank_data, reg_data) =
        interpret_motion(&mut ctx, Operator::Nop, Motion::NextSubWord, None, 2);

    assert!(yank_data.inclusive);
    assert_eq!(reg_data, RegisterData::char("word1 word2".to_owned()));
}

#[test]
fn interpret_motion_calls_delete_specific_fixes_for_delete_operator() {
    let mut ctx = setup("abc\ndef", Coords::default());

    let (yank_data, _) = interpret_motion(
        &mut ctx,
        Operator::Immediate(ImmediateOp::Delete),
        Motion::Line,
        Some(MotionMode::Charwise),
        1,
    );

    assert_eq!(yank_data.mode, MotionMode::Charwise);
}

#[test]
fn interpret_motion_calls_charwise_linewise_fix() {
    let mut ctx = setup("  abc\ndef  ", Coords::default());

    let (yank_data, reg_data) = interpret_motion(
        &mut ctx,
        Operator::Immediate(ImmediateOp::Delete),
        Motion::EndSubWord,
        Some(MotionMode::Charwise),
        2,
    );

    assert_eq!(yank_data.mode, MotionMode::Linewise);
    assert_eq!(yank_data.inclusive, true);
    assert_eq!(reg_data, RegisterData::line("  abc\ndef  \n".into()));
}

#[test]
fn interpret_motion_change_keeps_next_big_word_on_whitespace() {
    let mut ctx = setup("  alpha beta", Coords::default());

    let (yank_data, _) = interpret_motion(
        &mut ctx,
        Operator::Interactive(InteractiveOp::Change),
        Motion::NextBigWord,
        None,
        1,
    );

    assert_eq!(yank_data.end, Coords::new(0, 2));
    assert!(!yank_data.inclusive);
}

#[test]
fn interpret_motion_change_ends_big_word_from_a_non_whitespace_cursor() {
    let mut ctx = setup("  alpha beta", Coords::new(0, 2));

    let (yank_data, reg_data) = interpret_motion(
        &mut ctx,
        Operator::Interactive(InteractiveOp::Change),
        Motion::NextBigWord,
        None,
        1,
    );

    assert_eq!(yank_data.end, Coords::new(0, 6));
    assert!(yank_data.inclusive);
    assert_eq!(reg_data, RegisterData::char("alpha".to_owned()));
}

#[test]
fn interpret_motion_change_keeps_next_sub_word_on_whitespace() {
    let mut ctx = setup("  alpha_beta tail", Coords::default());

    let (yank_data, _) = interpret_motion(
        &mut ctx,
        Operator::Interactive(InteractiveOp::Change),
        Motion::NextSubWord,
        None,
        1,
    );

    assert_eq!(yank_data.end, Coords::new(0, 2));
    assert!(!yank_data.inclusive);
}

#[test]
fn interpret_motion_change_ends_big_word_for_next_sub_word_from_word_text() {
    let mut ctx = setup("alpha_beta tail", Coords::default());

    let (yank_data, reg_data) = interpret_motion(
        &mut ctx,
        Operator::Interactive(InteractiveOp::Change),
        Motion::NextSubWord,
        None,
        1,
    );

    assert_eq!(yank_data.end, Coords::new(0, 9));
    assert!(yank_data.inclusive);
    assert_eq!(reg_data, RegisterData::char("alpha_beta".to_owned()));
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

// --- apply_exceptions ---

#[test]
fn apply_exceptions_does_nothing_for_inclusive_ranges() {
    let mut ctx = setup("  abc\ndef", Coords::default());
    let mut yank_data = YankData::new(
        Coords::new(0, 2),
        Coords::new(1, 0),
        MotionMode::Charwise,
        true,
    );
    let original = yank_data;

    apply_exceptions(&mut ctx, &mut yank_data);

    assert_eq!(yank_data, original);
}

#[test]
fn apply_exceptions_does_nothing_when_end_is_not_at_column_zero() {
    let mut ctx = setup("abc\ndef", Coords::default());
    let mut yank_data = YankData::new(
        Coords::default(),
        Coords::new(1, 1),
        MotionMode::Charwise,
        false,
    );
    let original = yank_data;

    apply_exceptions(&mut ctx, &mut yank_data);

    assert_eq!(yank_data, original);
}

#[test]
fn apply_exceptions_does_nothing_when_end_is_not_on_a_later_row() {
    let mut ctx = setup("abc", Coords::default());
    let mut yank_data = YankData::new(
        Coords::new(0, 1),
        Coords::new(0, 1),
        MotionMode::Charwise,
        false,
    );
    let original = yank_data;

    apply_exceptions(&mut ctx, &mut yank_data);

    assert_eq!(yank_data, original);
}

#[test]
fn apply_exceptions_includes_the_upper_line_endpoint() {
    let mut ctx = setup("xabc\ndef", Coords::default());
    let mut yank_data = YankData::new(
        Coords::new(0, 1),
        Coords::new(1, 0),
        MotionMode::Charwise,
        false,
    );

    apply_exceptions(&mut ctx, &mut yank_data);

    assert_eq!(yank_data.end, Coords::new(0, 3));
    assert!(yank_data.inclusive);
    assert_eq!(yank_data.mode, MotionMode::Charwise);
}

#[test]
fn apply_exceptions_promotes_blank_prefix_ranges_to_linewise() {
    let mut ctx = setup("  abc\ndef", Coords::default());
    let mut yank_data = YankData::new(
        Coords::new(0, 2),
        Coords::new(1, 0),
        MotionMode::Charwise,
        false,
    );

    apply_exceptions(&mut ctx, &mut yank_data);

    assert_eq!(yank_data.end, Coords::new(0, 4));
    assert!(yank_data.inclusive);
    assert_eq!(yank_data.mode, MotionMode::Linewise);
}

// --- fix_line_charwise_d ---

#[test]
fn fix_line_charwise_d_does_nothing_for_non_line_motions() {
    let mut ctx = setup("abc\n", Coords::new(0, 2));
    let mut yank_data = YankData::new(
        Coords::new(0, 0),
        Coords::new(0, 3),
        MotionMode::Charwise,
        false,
    );
    let original = yank_data;

    fix_line_charwise_d(&mut ctx, Motion::Right, &mut yank_data);

    assert_eq!(yank_data, original);
}

#[test]
fn fix_line_charwise_d_does_nothing_for_non_charwise_modes() {
    let mut ctx = setup("abc\n", Coords::new(0, 2));
    let mut yank_data = YankData::new(
        Coords::new(0, 0),
        Coords::new(0, 3),
        MotionMode::Linewise,
        false,
    );
    let original = yank_data;

    fix_line_charwise_d(&mut ctx, Motion::Line, &mut yank_data);

    assert_eq!(yank_data, original);
}

#[test]
fn fix_line_charwise_d_makes_range_inclusive_at_newline() {
    let mut ctx = setup("abc\ndef", Coords::new(0, 0));
    let mut yank_data = YankData::new(
        Coords::new(0, 0),
        Coords::new(1, 0),
        MotionMode::Charwise,
        false,
    );

    fix_line_charwise_d(&mut ctx, Motion::Line, &mut yank_data);

    assert_eq!(yank_data.start, Coords::new(0, 0));
    assert_eq!(yank_data.end, Coords::new(0, 2));
    assert!(yank_data.inclusive);
}

#[test]
fn fix_line_charwise_d_resets_range_when_endpoint_is_before_newline() {
    let mut ctx = setup("abc", Coords::new(0, 2));
    let mut yank_data = YankData::new(
        Coords::new(0, 1),
        Coords::new(0, 2),
        MotionMode::Charwise,
        false,
    );

    fix_line_charwise_d(&mut ctx, Motion::Line, &mut yank_data);

    assert_eq!(yank_data.start, Coords::new(0, 0));
    assert_eq!(yank_data.end, Coords::new(0, 1));
    let (_, buf_view) = active_session!(ctx);
    assert_eq!(buf_view.cursor, Coords::new(0, 0));
}

// --- fix_d ---

#[test]
fn fix_d_does_nothing_for_non_charwise_ranges() {
    let mut ctx = setup("first\n   ", Coords::default());
    let mut yank_data = YankData::new(
        Coords::default(),
        Coords::new(1, 2),
        MotionMode::Linewise,
        false,
    );
    let original = yank_data;

    fix_d(&mut ctx, &mut yank_data);

    assert_eq!(yank_data, original);
}

#[test]
fn fix_d_does_nothing_for_ranges_within_one_row() {
    let mut ctx = setup("first", Coords::default());
    let mut yank_data = YankData::new(
        Coords::new(0, 0),
        Coords::new(0, 2),
        MotionMode::Charwise,
        false,
    );
    let original = yank_data;

    fix_d(&mut ctx, &mut yank_data);

    assert_eq!(yank_data, original);
}

#[test]
fn fix_d_keeps_charwise_when_text_before_start_is_not_whitespace() {
    let mut ctx = setup("xfirst\n   ", Coords::default());
    let mut yank_data = YankData::new(
        Coords::new(0, 1),
        Coords::new(1, 2),
        MotionMode::Charwise,
        false,
    );

    fix_d(&mut ctx, &mut yank_data);

    assert_eq!(yank_data.mode, MotionMode::Charwise);
}

#[test]
fn fix_d_keeps_charwise_when_text_after_end_is_not_whitespace() {
    let mut ctx = setup("first\nxabc", Coords::default());
    let mut yank_data = YankData::new(
        Coords::default(),
        Coords::new(1, 0),
        MotionMode::Charwise,
        false,
    );

    fix_d(&mut ctx, &mut yank_data);

    assert_eq!(yank_data.mode, MotionMode::Charwise);
}

#[test]
fn fix_d_promotes_blank_ranges_ending_at_eof() {
    let mut ctx = setup("  first\n   ", Coords::default());
    let mut yank_data = YankData::new(
        Coords::new(0, 2),
        Coords::new(1, 2),
        MotionMode::Charwise,
        false,
    );

    fix_d(&mut ctx, &mut yank_data);

    assert_eq!(yank_data.mode, MotionMode::Linewise);
    assert_eq!(yank_data.inclusive, true);
}

#[test]
fn fix_d_promotes_blank_ranges_ending_before_another_row() {
    let mut ctx = setup("  first\n   \nnext", Coords::default());
    let mut yank_data = YankData::new(
        Coords::new(0, 2),
        Coords::new(1, 2),
        MotionMode::Charwise,
        false,
    );

    fix_d(&mut ctx, &mut yank_data);

    assert_eq!(yank_data.mode, MotionMode::Linewise);
    assert_eq!(yank_data.inclusive, true);
}
