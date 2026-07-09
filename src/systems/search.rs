use anyhow::Result;
use ropey::Rope;

use crate::{
    cmd::{SearchOp, Secondary, Target},
    components::{Buffer, BufferView, Config, EditorCtx, EditorState, LastSearch, Session},
    rope,
    systems::commons::{char_idx_to_coords, cursor_to_char_idx, snap_coords},
};

pub struct SearchArgs {
    pub op: SearchOp,
    pub reps: usize,
    pub target: Target,
}

impl SearchArgs {
    pub fn new(op: SearchOp, reps: usize, target: Target) -> Self {
        Self { op, reps, target }
    }

    fn as_char(&self) -> Option<char> {
        match self.target {
            Target::Secondary(Secondary::Char(c)) => Some(c),
            _ => None,
        }
    }
}

pub fn handle_search(ctx: &EditorCtx, args: SearchArgs) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;

    let mut q_session = ctx
        .world
        .query_one::<(&Session, &mut BufferView)>(editor.session_id);
    let (session, buf_view) = q_session.get()?;
    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;

    let mut q_search = ctx.world.query::<&mut LastSearch>();
    let last_search = q_search.iter().next().expect("No search data");

    match args.op {
        SearchOp::FindNextChar => {
            if let Some(c) = args.as_char() {
                last_search.set_char(c, args.op);
                find_char_forward(&config, &buffer.rope, buf_view, c, args.reps);
            }
        }
        SearchOp::FindPrevChar => {
            if let Some(c) = args.as_char() {
                last_search.set_char(c, args.op);
                find_char_backward(&config, &buffer.rope, buf_view, c, args.reps);
            }
        }
        SearchOp::TillNextChar => {
            if let Some(c) = args.as_char() {
                last_search.set_char(c, args.op);
                till_char_forward(&config, &buffer.rope, buf_view, c, args.reps, false);
            }
        }
        SearchOp::TillPrevChar => {
            if let Some(c) = args.as_char() {
                last_search.set_char(c, args.op);
                till_char_backward(&config, &buffer.rope, buf_view, c, args.reps, false);
            }
        }
        SearchOp::RepeatForward => {
            repeat_forward(&config, &buffer.rope, buf_view, last_search, args.reps)
        }
        SearchOp::RepeatBackward => {
            repeat_backward(&config, &buffer.rope, buf_view, last_search, args.reps)
        }
    }

    Ok(())
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
    buf_view: &mut BufferView,
    last_search: &LastSearch,
    reps: usize,
) {
    if let Some(char_search) = last_search.char_search.as_ref() {
        let c = char_search.c;
        match char_search.op {
            SearchOp::FindNextChar => find_char_forward(config, rope, buf_view, c, reps),
            SearchOp::FindPrevChar => find_char_backward(config, rope, buf_view, c, reps),
            SearchOp::TillNextChar => till_char_forward(config, rope, buf_view, c, reps, true),
            SearchOp::TillPrevChar => till_char_backward(config, rope, buf_view, c, reps, true),
            _ => {}
        }
    }
}

fn repeat_backward(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    last_search: &LastSearch,
    reps: usize,
) {
    if let Some(char_search) = last_search.char_search.as_ref() {
        let c = char_search.c;
        match char_search.op {
            SearchOp::FindNextChar => find_char_backward(config, rope, buf_view, c, reps),
            SearchOp::FindPrevChar => find_char_forward(config, rope, buf_view, c, reps),
            SearchOp::TillNextChar => till_char_backward(config, rope, buf_view, c, reps, true),
            SearchOp::TillPrevChar => till_char_forward(config, rope, buf_view, c, reps, true),
            _ => {}
        }
    }
}
