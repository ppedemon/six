use ropey::RopeSlice;

use crate::{
    active_session_and_buffer,
    components::{Buffer, BufferName, BufferView, EditorCtx, Level, Session, Status},
    ex::{ExError, ExRange, solve_exrange},
    misc, rope,
    systems::event::on_buffer_saved,
};

pub fn save_active(
    ctx: &mut EditorCtx,
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    range: ExRange,
) -> Result<(), ExError> {
    let status = &mut ctx.status;
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    save(
        (session, buf_view, buffer),
        status,
        name,
        append,
        only_if_dirty,
        range,
    )?;

    Ok(())
}

pub fn hard_save_active(
    ctx: &mut EditorCtx,
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    range: ExRange,
) -> Result<(), ExError> {
    let status = &mut ctx.status;
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    hard_save(
        (session, buf_view, buffer),
        status,
        name,
        append,
        only_if_dirty,
        range,
    )?;

    Ok(())
}

pub fn save_all(ctx: &mut EditorCtx, only_if_dirty: bool) -> Result<(), ExError> {
    let status = &mut ctx.status;

    for (session, buf_view) in ctx.sessions.values_mut() {
        let buffer = ctx.buffers.get_mut(&session.buf_id).unwrap();
        save(
            (session, buf_view, buffer),
            status,
            None,
            false,
            only_if_dirty,
            ExRange::All,
        )?;
    }
    ctx.status.set_msg(Level::Info, "All buffers written");
    Ok(())
}

pub fn hard_save_all(ctx: &mut EditorCtx, only_if_dirty: bool) -> Result<(), ExError> {
    let status = &mut ctx.status;

    for (session, buf_view) in ctx.sessions.values_mut() {
        let buffer = ctx.buffers.get_mut(&session.buf_id).unwrap();
        hard_save(
            (session, buf_view, buffer),
            status,
            None,
            false,
            only_if_dirty,
            ExRange::All,
        )?;
    }
    ctx.status.set_msg(Level::Info, "All buffers written");
    Ok(())
}

fn save(
    session_args: (&mut Session, &BufferView, &mut Buffer),
    status: &mut Status,
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    range: ExRange,
) -> Result<(), ExError> {
    let (session, buf_view, buffer) = session_args;
    let curr_line = buf_view.cursor.row;

    if only_if_dirty && !buffer.is_dirty() {
        return Ok(());
    }

    let range = range.coerce_implicit_to(ExRange::All);
    let rope_slice = rope_slice(&buffer, curr_line, range)?;
    let writes_all = rope_slice.len_lines() == buffer.rope().len_lines();

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
            save_buffer(status, orig_name, append, rope_slice)?;
            buffer.saved();
            Ok(())
        }
        (Some(given_name), None) => {
            save_buffer(status, given_name, append, rope_slice)?;
            if !append && writes_all {
                session.buf_name = name;
                buffer.saved();
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
            save_buffer(status, given_name, append, rope_slice)?;
            if given_name.file_path.as_path() == orig_name.file_path.as_path() {
                buffer.saved();
            }
            Ok(())
        }
    }
}

fn hard_save(
    session_args: (&mut Session, &BufferView, &mut Buffer),
    status: &mut Status,
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    range: ExRange,
) -> Result<(), ExError> {
    let (session, buf_view, buffer) = session_args;
    let curr_line = buf_view.cursor.row;

    if only_if_dirty && !buffer.is_dirty() {
        return Ok(());
    }

    let range = range.coerce_implicit_to(ExRange::All);
    let rope_slice = rope_slice(&buffer, curr_line, range)?;
    let writes_all = rope_slice.len_lines() == buffer.rope().len_lines();

    match (name.as_ref(), session.buf_name.as_ref()) {
        (None, None) => Err(ExError::NoFileName),
        (None, Some(orig_name)) => {
            hard_save_buffer(status, orig_name, append, rope_slice)?;
            if writes_all {
                buffer.saved();
            }
            Ok(())
        }
        (Some(given_name), _) => {
            hard_save_buffer(status, given_name, append, rope_slice)?;
            if !append && session.buf_name.is_none() {
                session.buf_name = name;
            }
            if writes_all {
                buffer.saved();
            }
            Ok(())
        }
    }
}

fn rope_slice(buffer: &Buffer, curr_line: usize, range: ExRange) -> Result<RopeSlice<'_>, ExError> {
    let range = solve_exrange(range, &buffer.rope(), curr_line)?;
    Ok(rope::slice_as_view(buffer.rope(), range))
}

fn save_buffer(
    status: &mut Status,
    name: &BufferName,
    append: bool,
    rope_slice: RopeSlice<'_>,
) -> Result<(), ExError> {
    on_buffer_saved(status, name, rope_slice);
    misc::io::save(name.file_path.as_path(), append, rope_slice)?;
    Ok(())
}

fn hard_save_buffer(
    status: &mut Status,
    name: &BufferName,
    append: bool,
    rope_slice: RopeSlice<'_>,
) -> Result<(), ExError> {
    on_buffer_saved(status, name, rope_slice);
    misc::io::hard_save(name.file_path.as_path(), append, rope_slice)?;
    Ok(())
}
