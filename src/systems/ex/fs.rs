use hecs::Entity;
use ropey::RopeSlice;

use crate::{
    components::{Buffer, BufferName, BufferView, EditorCtx, EditorState, Level, Session},
    ex::{ExError, ExRange, solve_exrange},
    misc, rope,
    systems::{event::on_buffer_saved, status},
};

pub fn save_active(
    ctx: &EditorCtx,
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    range: ExRange,
) -> Result<(), ExError> {
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id).unwrap();
    let mut session = ctx.world.get::<&mut Session>(editor.session_id).unwrap();
    save(
        ctx,
        (editor.session_id, &mut session),
        name,
        append,
        only_if_dirty,
        range,
    )?;
    Ok(())
}

pub fn hard_save_active(
    ctx: &EditorCtx,
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    range: ExRange,
) -> Result<(), ExError> {
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id).unwrap();
    let mut session = ctx.world.get::<&mut Session>(editor.session_id).unwrap();
    hard_save(
        ctx,
        (editor.session_id, &mut session),
        name,
        append,
        only_if_dirty,
        range,
    )?;
    Ok(())
}

pub fn save_all(ctx: &EditorCtx, only_if_dirty: bool) -> Result<(), ExError> {
    let mut q_session = ctx.world.query::<(Entity, &mut Session)>();
    for (session_id, session) in q_session.iter() {
        save(
            ctx,
            (session_id, session),
            None,
            false,
            only_if_dirty,
            ExRange::All,
        )?;
    }
    status::set_msg(ctx, Level::Info, "All buffers written").unwrap();
    Ok(())
}

pub fn hard_save_all(ctx: &EditorCtx, only_if_dirty: bool) -> Result<(), ExError> {
    let mut q_session = ctx.world.query::<(Entity, &mut Session)>();
    for (session_id, session) in q_session.iter() {
        hard_save(
            ctx,
            (session_id, session),
            None,
            false,
            only_if_dirty,
            ExRange::All,
        )?;
    }
    status::set_msg(ctx, Level::Info, "All buffers written").unwrap();
    Ok(())
}

fn save(
    ctx: &EditorCtx,
    session_args: (Entity, &mut Session),
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    range: ExRange,
) -> Result<(), ExError> {
    let (session_id, session) = session_args;
    let buf_view = ctx.world.get::<&BufferView>(session_id).unwrap();
    let curr_line = buf_view.cursor.row;
    let mut buffer = ctx.world.get::<&mut Buffer>(session.buf_id).unwrap();

    if only_if_dirty && !buffer.dirty {
        return Ok(());
    }

    let range = range.coerce_implicit_to(ExRange::All);
    let rope_slice = rope_slice(&buffer, curr_line, range)?;
    let writes_all = rope_slice.len_lines() == buffer.rope.len_lines();

    match (name.as_ref(), session.buf_name.as_ref()) {
        (None, None) => Err(ExError::NoFileName),
        (None, Some(orig_name)) => {
            // Can't append to myself
            if append {
                return Err(ExError::FileExists);
            }
            // Can't do a partial write to myself
            if !writes_all {
                return Err(ExError::PartialWrite);
            }
            save_buffer(ctx, orig_name, append, rope_slice)?;
            buffer.dirty = false;
            Ok(())
        }
        (Some(given_name), None) => {
            save_buffer(ctx, given_name, append, rope_slice)?;
            if !append && writes_all {
                session.buf_name = name;
                buffer.dirty = false;
            }
            Ok(())
        }
        (Some(given_name), Some(orig_name)) => {
            if given_name.file_path.as_path() == orig_name.file_path.as_path() {
                // Can't append to myself
                if append {
                    return Err(ExError::FileExists);
                }
                // Can't do a partial write to myself
                if !writes_all {
                    return Err(ExError::PartialWrite);
                }
            }
            save_buffer(ctx, given_name, append, rope_slice)?;
            if given_name.file_path.as_path() == orig_name.file_path.as_path() {
                buffer.dirty = false;
            }
            Ok(())
        }
    }
}

fn hard_save(
    ctx: &EditorCtx,
    session_args: (Entity, &mut Session),
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    range: ExRange,
) -> Result<(), ExError> {
    let (session_id, session) = session_args;
    let buf_view = ctx.world.get::<&BufferView>(session_id).unwrap();
    let curr_line = buf_view.cursor.row;
    let mut buffer = ctx.world.get::<&mut Buffer>(session.buf_id).unwrap();

    if only_if_dirty && !buffer.dirty {
        return Ok(());
    }

    let range = range.coerce_implicit_to(ExRange::All);
    let rope_slice = rope_slice(&buffer, curr_line, range)?;
    let writes_all = rope_slice.len_lines() == buffer.rope.len_lines();

    match (name.as_ref(), session.buf_name.as_ref()) {
        (None, None) => Err(ExError::NoFileName),
        (None, Some(orig_name)) => {
            hard_save_buffer(ctx, orig_name, append, rope_slice)?;
            if writes_all {
                buffer.dirty = false;
            }
            Ok(())
        }
        (Some(given_name), _) => {
            hard_save_buffer(ctx, given_name, append, rope_slice)?;
            if !append && session.buf_name.is_none() {
                session.buf_name = name;
            }
            if writes_all {
                buffer.dirty = false;
            }
            Ok(())
        }
    }
}

fn rope_slice(buffer: &Buffer, curr_line: usize, range: ExRange) -> Result<RopeSlice<'_>, ExError> {
    let range = solve_exrange(range, &buffer.rope, curr_line)?;
    Ok(rope::slice_as_view(&buffer.rope, range))
}

fn save_buffer(
    ctx: &EditorCtx,
    name: &BufferName,
    append: bool,
    rope_slice: RopeSlice<'_>,
) -> Result<(), ExError> {
    on_buffer_saved(ctx, name, rope_slice).unwrap();
    misc::io::save(name.file_path.as_path(), append, rope_slice)?;
    Ok(())
}

fn hard_save_buffer(
    ctx: &EditorCtx,
    name: &BufferName,
    append: bool,
    rope_slice: RopeSlice<'_>,
) -> Result<(), ExError> {
    on_buffer_saved(ctx, name, rope_slice).unwrap();
    misc::io::hard_save(name.file_path.as_path(), append, rope_slice)?;
    Ok(())
}
