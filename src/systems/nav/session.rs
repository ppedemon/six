use anyhow::Result;
use hecs::Entity;
use ropey::Rope;

use super::{
    buffer,
    rules::{InsertNav, NavRules, NormalNav},
};
use crate::components::{
    Buffer, BufferView, Config, EditorCtx, EditorState, ExSession, Focus, LastSearch, Mode,
    Session, Viewport,
};
use crate::{
    cmd::{Cmd, Motion},
    rope,
    systems::commons::{char_idx_to_coords, cursor_to_char_idx, snap_coords},
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

pub fn handle_nav(ctx: &EditorCtx, nav_args: NavArgs) -> Result<()> {
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
    match editor.focus {
        Focus::Ex => handle_ex_nav(ctx, nav_args),
        Focus::Session => handle_session_nav(ctx, editor.session_id, nav_args),
    }
}

fn handle_ex_nav(ctx: &EditorCtx, args: NavArgs) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut q_ex = ctx
        .world
        .query_one::<(&ExSession, &mut BufferView)>(ctx.ex_session_id);
    let (ex_session, buf_view) = q_ex.get()?;

    let reps = args.cmd.reps.unwrap_or(1);

    match args.motion {
        Motion::Left => {
            if buf_view.cursor.col > 1 {
                buffer::move_left::<InsertNav>(&config, &ex_session.rope, buf_view, reps);
            }
        }
        Motion::Right => {
            buffer::move_right::<InsertNav>(&config, &ex_session.rope, buf_view, reps);
        }
        _ => {}
    }

    Ok(())
}

fn handle_session_nav(ctx: &EditorCtx, session_id: Entity, args: NavArgs) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let mut last_search = ctx.world.get::<&mut LastSearch>(ctx.search_id)?;

    let mut q_session = ctx
        .world
        .query_one::<(&mut Session, &mut BufferView)>(session_id);
    let (session, buf_view) = q_session.get()?;
    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;

    match session.mode {
        Mode::Insert => {
            session.insert_log.reset();
            session_nav::<InsertNav>(
                &config,
                &buffer.rope,
                &mut last_search,
                &mut session.viewport,
                buf_view,
                args,
            );
        }
        Mode::Normal => {
            session_nav::<NormalNav>(
                &config,
                &buffer.rope,
                &mut last_search,
                &mut session.viewport,
                buf_view,
                args,
            );
        }
    }

    Ok(())
}

const PAGE_SCROLL_MARGIN: u16 = 3;

fn session_nav<R: NavRules>(
    config: &Config,
    rope: &Rope,
    last_search: &mut LastSearch,
    viewport: &mut Viewport,
    buf_view: &mut BufferView,
    args: NavArgs,
) {
    let reps = args.cmd.reps.unwrap_or(1);

    match args.motion {
        Motion::Up => buffer::move_up::<R>(config, rope, buf_view, reps),
        Motion::Down => buffer::move_down::<R>(config, rope, buf_view, reps),
        Motion::Left => buffer::move_left::<R>(config, rope, buf_view, reps),
        Motion::Right => buffer::move_right::<R>(config, rope, buf_view, reps),
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
        Motion::NextBigWord => buffer::next_big_word(config, rope, buf_view, reps),
        Motion::NextSubWord => buffer::next_sub_word(config, rope, buf_view, reps),
        Motion::PrevBigWord => buffer::prev_big_word(config, rope, buf_view, reps),
        Motion::PrevSubWord => buffer::prev_sub_word(config, rope, buf_view, reps),
        Motion::EndBigWord => buffer::end_big_word(config, rope, buf_view, reps),
        Motion::EndSubWord => buffer::end_sub_word(config, rope, buf_view, reps),

        Motion::FindNextChar(c) => {
            last_search.save_char_search(args.motion);
            find_char_forward(config, rope, buf_view, c, reps);
        }
        Motion::FindPrevChar(c) => {
            last_search.save_char_search(args.motion);
            find_char_backward(config, rope, buf_view, c, reps);
        }
        Motion::TillNextChar(c) => {
            last_search.save_char_search(args.motion);
            till_char_forward(config, rope, buf_view, c, reps, false);
        }
        Motion::TillPrevChar(c) => {
            last_search.save_char_search(args.motion);
            till_char_backward(config, rope, buf_view, c, reps, false);
        }
        Motion::RepeatForward => repeat_forward(config, rope, &last_search, buf_view, reps),
        Motion::RepeatBackward => repeat_backward(config, rope, last_search, buf_view, reps),

        Motion::FirstNonBlankInLine => buffer::line_first_non_blank::<R>(config, rope, buf_view),
        Motion::StartOfLine => buffer::start_of_line::<R>(config, rope, buf_view),
        Motion::EndOfLine => buffer::end_of_line::<R>(config, rope, buf_view),
        Motion::FirstNonBlankInFile => buffer::file_first_non_blank::<R>(config, rope, buf_view),
        Motion::StartOfFile => buffer::start_of_file::<R>(config, rope, buf_view),
        Motion::EndOfFile => buffer::end_of_file::<R>(config, rope, buf_view),

        Motion::GotoLine(line) => buffer::goto_line::<R>(config, rope, buf_view, line),
        Motion::Line => {}
    }
}

// On startup, move cursor to the first non-blank character of the active session
pub fn init_cursor_pos(ctx: &EditorCtx) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;

    let mut q_session = ctx
        .world
        .query_one::<(&Session, &mut BufferView)>(editor.session_id);
    let (session, buf_view) = q_session.get()?;

    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;
    buffer::file_first_non_blank::<NormalNav>(&config, &buffer.rope, buf_view);

    Ok(())
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

fn find_char_forward(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    c: char,
    reps: usize,
) {
    let mut char_idx = cursor_to_char_idx(config, buf_view, rope);
    char_idx = rope::find_char_forward(rope, c, reps, char_idx);
    let coords = char_idx_to_coords(config, rope, buf_view, char_idx);
    snap_coords(config, rope, buf_view, coords);
}

fn find_char_backward(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    c: char,
    reps: usize,
) {
    let mut char_idx = cursor_to_char_idx(config, buf_view, rope);
    char_idx = rope::find_char_backward(rope, c, reps, char_idx);
    let coords = char_idx_to_coords(config, rope, buf_view, char_idx);
    snap_coords(config, rope, buf_view, coords);
}

fn till_char_forward(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    c: char,
    reps: usize,
    repeats_last: bool,
) {
    let mut char_idx = cursor_to_char_idx(config, buf_view, rope);

    let mut n = reps;
    if repeats_last && char_idx < rope.len_chars().saturating_sub(1) && rope.char(char_idx + 1) == c
    {
        n += 1;
    }

    char_idx = rope::till_char_forward(rope, c, n, char_idx);
    let coords = char_idx_to_coords(config, rope, buf_view, char_idx);
    snap_coords(config, rope, buf_view, coords);
}

fn till_char_backward(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    c: char,
    reps: usize,
    repeats_last: bool,
) {
    let mut char_idx = cursor_to_char_idx(config, buf_view, rope);

    let mut n = reps;
    if repeats_last && char_idx > 0 && rope.char(char_idx - 1) == c {
        n += 1;
    }

    char_idx = rope::till_char_backward(rope, c, n, char_idx);
    let coords = char_idx_to_coords(config, rope, buf_view, char_idx);
    snap_coords(config, rope, buf_view, coords);
}

fn repeat_forward(
    config: &Config,
    rope: &Rope,
    last_search: &LastSearch,
    buf_view: &mut BufferView,
    reps: usize,
) {
    if let Some(&m) = last_search.get_char_search().as_ref() {
        match m {
            Motion::FindNextChar(c) => find_char_forward(config, rope, buf_view, c, reps),
            Motion::FindPrevChar(c) => find_char_backward(config, rope, buf_view, c, reps),
            Motion::TillNextChar(c) => till_char_forward(config, rope, buf_view, c, reps, true),
            Motion::TillPrevChar(c) => till_char_backward(config, rope, buf_view, c, reps, true),
            _ => {}
        }
    }
}

fn repeat_backward(
    config: &Config,
    rope: &Rope,
    last_search: &LastSearch,
    buf_view: &mut BufferView,
    reps: usize,
) {
    if let Some(&m) = last_search.get_char_search().as_ref() {
        match m {
            Motion::FindNextChar(c) => find_char_backward(config, rope, buf_view, c, reps),
            Motion::FindPrevChar(c) => find_char_forward(config, rope, buf_view, c, reps),
            Motion::TillNextChar(c) => till_char_backward(config, rope, buf_view, c, reps, true),
            Motion::TillPrevChar(c) => till_char_forward(config, rope, buf_view, c, reps, true),
            _ => {}
        }
    }
}
