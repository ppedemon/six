use anyhow::Result;
use ropey::Rope;

use crate::{
    cmd::{SearchOp, Secondary, Target},
    components::{Buffer, BufferView, Config, EditorCtx, EditorState, Session},
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
}

pub fn handle_search(ctx: &EditorCtx, args: SearchArgs) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
    let mut q_session = ctx
        .world
        .query_one::<(&Session, &mut BufferView)>(editor.session_id);
    let (session, buf_view) = q_session.get()?;
    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;

    match args.op {
        SearchOp::FindNextChar => {
            if let Target::Secondary(Secondary::Char(c)) = args.target {
                find_char_forward(&config, &buffer.rope, buf_view, c, args.reps);
            }
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

    for _ in 0..reps {
        char_idx = rope::find_char_forward(rope, c, char_idx);
    }

    let coords = char_idx_to_coords(config, rope, buf_view, char_idx);
    snap_coords(config, rope, buf_view, coords);
}
