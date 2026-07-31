use ropey::Rope;

use super::{
    buffer,
    rules::{InsertNav, NavRules, NormalNav},
};
use crate::{
    active_session_and_buffer,
    cmd::{Cmd, Motion},
    components::{Buffer, BufferView, Config, EditorCtx, Focus, LastNav, Mode, Viewport},
    systems::commons::char_idx_to_coords,
};

pub struct NavArgs {
    motion: Motion,
    cmd: Cmd,
}

impl NavArgs {
    pub fn new(motion: Motion, cmd: Cmd) -> Self {
        Self { motion, cmd }
    }
}

pub fn handle_nav(ctx: &mut EditorCtx, nav_args: NavArgs) {
    match ctx.editor.focus {
        Focus::Ex => handle_ex_nav(ctx, nav_args),
        Focus::Session => handle_session_nav(ctx, nav_args),
    }
}

fn handle_ex_nav(ctx: &mut EditorCtx, args: NavArgs) {
    let reps = args.cmd.reps.unwrap_or(1);
    let buf_view = &mut ctx.ex_buffer_view;

    match args.motion {
        Motion::Left => {
            if buf_view.cursor.col > 1 {
                buffer::move_left::<InsertNav>(&ctx.config, ctx.ex_session.rope(), buf_view, reps);
            }
        }
        Motion::Right => {
            buffer::move_right::<InsertNav>(&ctx.config, &ctx.ex_session.rope(), buf_view, reps);
        }
        _ => {}
    }
}

pub fn handle_session_nav(ctx: &mut EditorCtx, args: NavArgs) {
    let config = &ctx.config;
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    match session.mode {
        Mode::Insert => {
            session.insert_log.reset();
            session_nav::<InsertNav>(
                config,
                buffer,
                &mut ctx.last_nav,
                &mut session.viewport,
                buf_view,
                args,
            );
        }
        Mode::Normal => {
            session_nav::<NormalNav>(
                &ctx.config,
                buffer,
                &mut ctx.last_nav,
                &mut session.viewport,
                buf_view,
                args,
            );
        }
    }
}

const PAGE_SCROLL_MARGIN: u16 = 3;

fn session_nav<R: NavRules>(
    config: &Config,
    buffer: &Buffer,
    last_nav: &mut LastNav,
    viewport: &mut Viewport,
    buf_view: &mut BufferView,
    args: NavArgs,
) {
    let reps = args.cmd.reps.unwrap_or(1);
    let rope = buffer.rope();

    last_nav.clear_overshot();

    match args.motion {
        Motion::Up => buffer::move_up::<R>(config, rope, buf_view, reps),
        Motion::Down => buffer::move_down::<R>(config, rope, buf_view, reps),
        Motion::Left => buffer::move_left::<R>(config, rope, buf_view, reps),
        Motion::Right => {
            let overshot = buffer::move_right::<R>(config, rope, buf_view, reps);
            if overshot {
                last_nav.save_overshot();
            }
        }
        Motion::PageUp => {
            let pg_size = viewport.pg_size(PAGE_SCROLL_MARGIN);
            buffer::page_up::<R>(config, rope, buf_view, reps, pg_size);
            viewport.scroll_to_row(buf_view.cursor.row);
        }
        Motion::PageDown => {
            let pg_size = viewport.pg_size(PAGE_SCROLL_MARGIN);
            buffer::page_down::<R>(config, rope, buf_view, reps, pg_size);
            viewport.scroll_to_row(buf_view.cursor.row);
        }
        Motion::NextBigWord => {
            let overshot = buffer::next_big_word(config, rope, buf_view, reps);
            if overshot {
                last_nav.save_overshot();
            }
        }
        Motion::NextSubWord => {
            let overshot = buffer::next_sub_word(config, rope, buf_view, reps);
            if overshot {
                last_nav.save_overshot();
            }
        }
        Motion::PrevBigWord => buffer::prev_big_word(config, rope, buf_view, reps),
        Motion::PrevSubWord => buffer::prev_sub_word(config, rope, buf_view, reps),
        Motion::EndBigWord => {
            let overshot = buffer::end_big_word(config, rope, buf_view, reps);
            if overshot {
                last_nav.save_overshot();
            }
        }
        Motion::EndSubWord => {
            let overshot = buffer::end_sub_word(config, rope, buf_view, reps);
            if overshot {
                last_nav.save_overshot();
            }
        }
        Motion::FindNextChar(c) => {
            last_nav.save_char_search(args.motion);
            buffer::find_char_forward(config, rope, buf_view, c, reps);
        }
        Motion::FindPrevChar(c) => {
            last_nav.save_char_search(args.motion);
            buffer::find_char_backward(config, rope, buf_view, c, reps);
        }
        Motion::TillNextChar(c) => {
            last_nav.save_char_search(args.motion);
            buffer::till_char_forward(config, rope, buf_view, c, reps, false);
        }
        Motion::TillPrevChar(c) => {
            last_nav.save_char_search(args.motion);
            buffer::till_char_backward(config, rope, buf_view, c, reps, false);
        }
        Motion::RepeatForward => repeat_forward(config, rope, &last_nav, buf_view, reps),
        Motion::RepeatBackward => repeat_backward(config, rope, last_nav, buf_view, reps),

        Motion::FirstNonBlankInLine => buffer::line_first_non_blank::<R>(config, rope, buf_view),
        Motion::StartOfLine => buffer::start_of_line::<R>(config, rope, buf_view),
        Motion::EndOfLine => buffer::end_of_line::<R>(config, rope, buf_view),
        Motion::FirstNonBlankInFile => buffer::file_first_non_blank::<R>(config, rope, buf_view),
        Motion::StartOfFile => buffer::start_of_file::<R>(config, rope, buf_view),
        Motion::EndOfFile => buffer::end_of_file::<R>(config, rope, buf_view),

        Motion::GotoLine(line) => goto_line::<R>(config, rope, viewport, buf_view, line),
        Motion::GotoMark(c) => goto_mark::<R>(config, buffer, viewport, buf_view, c),
        Motion::ExactGotoMark(c) => exact_goto_mark::<R>(config, buffer, viewport, buf_view, c),

        Motion::Line => line::<R>(buffer, buf_view, reps),
    }
}

// On startup, move cursor to the first non-blank character of the active session
pub fn init_cursor_pos(ctx: &mut EditorCtx) {
    let (session, buf_view) = ctx.sessions.get_mut(&ctx.editor.session_id).unwrap();
    let buffer = ctx.buffers.get(&session.buf_id).unwrap();
    buffer::file_first_non_blank::<NormalNav>(&ctx.config, buffer.rope(), buf_view);
}

fn repeat_forward(
    config: &Config,
    rope: &Rope,
    last_nav: &LastNav,
    buf_view: &mut BufferView,
    reps: usize,
) {
    if let Some(&m) = last_nav.last_char_search().as_ref() {
        match m {
            Motion::FindNextChar(c) => buffer::find_char_forward(config, rope, buf_view, c, reps),
            Motion::FindPrevChar(c) => buffer::find_char_backward(config, rope, buf_view, c, reps),
            Motion::TillNextChar(c) => {
                buffer::till_char_forward(config, rope, buf_view, c, reps, true)
            }
            Motion::TillPrevChar(c) => {
                buffer::till_char_backward(config, rope, buf_view, c, reps, true)
            }
            _ => {}
        }
    }
}

fn repeat_backward(
    config: &Config,
    rope: &Rope,
    last_nav: &LastNav,
    buf_view: &mut BufferView,
    reps: usize,
) {
    if let Some(&m) = last_nav.last_char_search().as_ref() {
        match m {
            Motion::FindNextChar(c) => buffer::find_char_backward(config, rope, buf_view, c, reps),
            Motion::FindPrevChar(c) => buffer::find_char_forward(config, rope, buf_view, c, reps),
            Motion::TillNextChar(c) => {
                buffer::till_char_backward(config, rope, buf_view, c, reps, true)
            }
            Motion::TillPrevChar(c) => {
                buffer::till_char_forward(config, rope, buf_view, c, reps, true)
            }
            _ => {}
        }
    }
}

pub fn line<R: NavRules>(buffer: &Buffer, buf_view: &mut BufferView, reps: usize) {
    let max_row = buffer.rope().len_lines().saturating_sub(1);
    buf_view.cursor.row = (buf_view.cursor.row + reps.saturating_sub(1)).min(max_row);
}

pub fn goto_line<R: NavRules>(
    config: &Config,
    rope: &Rope,
    viewport: &mut Viewport,
    buf_view: &mut BufferView,
    line: usize,
) {
    let old_line = buf_view.cursor.row;
    buffer::goto_line::<R>(config, rope, buf_view, line);

    let h = viewport.area.height.saturating_div(2) as usize;
    let scroll_start = viewport.scroll.row;
    let scroll_end = viewport.scroll.row + viewport.area.height as usize;

    if buf_view.cursor.row + h <= scroll_start
        || buf_view.cursor.row.saturating_sub(h) >= scroll_end
    {
        viewport.scroll.row = buf_view.cursor.row.saturating_sub(h).min(
            rope.len_lines()
                .saturating_sub(viewport.area.height as usize),
        );
    }
}

pub fn goto_mark<R: NavRules>(
    config: &Config,
    buffer: &Buffer,
    viewport: &mut Viewport,
    buf_view: &mut BufferView,
    mark: char,
) {
    if let Some(char_idx) = buffer.marks().read(mark) {
        let coord = char_idx_to_coords(config, buffer.rope(), buf_view, char_idx);
        goto_line::<R>(config, buffer.rope(), viewport, buf_view, coord.row + 1); // goto_line counts from 1
    }
}

pub fn exact_goto_mark<R: NavRules>(
    config: &Config,
    buffer: &Buffer,
    viewport: &mut Viewport,
    buf_view: &mut BufferView,
    mark: char,
) {
    if let Some(char_idx) = buffer.marks().read(mark) {
        let coord = char_idx_to_coords(config, buffer.rope(), buf_view, char_idx);
        goto_line::<R>(config, buffer.rope(), viewport, buf_view, coord.row + 1); // goto_line counts from 1
        buffer::goto_col::<R>(config, buffer.rope(), buf_view, coord.col);
    }
}
