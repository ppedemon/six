use anyhow::Result;
use hecs::QueryOne;
use ropey::Rope;

use crate::components::{
    BufferView, Config, Coords, DisplayLineRef, EditorCtx, EditorState, ExSession, Session,
};

pub fn curr_line<'a>(
    config: &Config,
    rope: &Rope,
    buf_view: &'a mut BufferView,
) -> DisplayLineRef<'a> {
    buf_view
        .display_buf
        .ensure_line(config, rope, buf_view.cursor.row)
}

pub fn cursor_to_char_idx(config: &Config, buf_view: &mut BufferView, rope: &Rope) -> usize {
    coords_to_char_idx(config, rope, buf_view, buf_view.cursor)
}

pub fn coords_to_char_idx(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    coords: Coords,
) -> usize {
    if rope.len_chars() == 0 {
        return 0;
    }

    let line_idx = rope.line_to_char(coords.row);
    let display_line = curr_line(config, rope, buf_view);
    let col_idx = display_line.col_to_rope_idx(coords.col);
    line_idx + col_idx
}

pub fn char_idx_to_coords(
    config: &Config,
    rope: &Rope,
    buf_view: &mut BufferView,
    char_idx: usize,
) -> Coords {
    let line_idx = rope.char_to_line(char_idx);
    let start_idx = rope.line_to_char(line_idx);
    let line = buf_view.display_buf.ensure_line(config, rope, line_idx);
    let col_idx = line.char_idx_to_col(char_idx - start_idx);

    Coords {
        row: line_idx,
        col: col_idx,
    }
}

pub fn snap_coords(config: &Config, rope: &Rope, buf_view: &mut BufferView, coords: Coords) {
    let line = buf_view.display_buf.ensure_line(config, rope, coords.row);
    let col = line.snap_col(coords.col);

    buf_view.cursor.row = coords.row;
    buf_view.cursor.col = col;
    buf_view.target_col = col;
}

//
// Convenience World operations
//

pub fn active_session_id(ctx: &EditorCtx) -> Result<hecs::Entity> {
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
    Ok(editor.session_id)
}

pub fn active_session_query<'a>(
    ctx: &'a EditorCtx,
) -> Result<QueryOne<'a, (&'a Session, &'a BufferView)>> {
    let session_id = active_session_id(ctx)?;
    Ok(ctx.world.query_one::<(&Session, &BufferView)>(session_id))
}

pub fn mut_active_session_query<'a>(
    ctx: &'a EditorCtx,
) -> Result<QueryOne<'a, (&'a mut Session, &'a mut BufferView)>> {
    let session_id = active_session_id(ctx)?;
    Ok(ctx
        .world
        .query_one::<(&mut Session, &mut BufferView)>(session_id))
}

pub fn ex_session_query<'a>(
    ctx: &'a EditorCtx,
) -> Result<QueryOne<'a, (&'a ExSession, &'a BufferView)>> {
    Ok(ctx
        .world
        .query_one::<(&ExSession, &BufferView)>(ctx.ex_session_id))
}

pub fn mut_ex_session_query<'a>(
    ctx: &'a EditorCtx,
) -> Result<QueryOne<'a, (&'a mut ExSession, &'a mut BufferView)>> {
    Ok(ctx
        .world
        .query_one::<(&mut ExSession, &mut BufferView)>(ctx.ex_session_id))
}
