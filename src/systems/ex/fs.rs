use std::ops::Range;

use crate::{
    active_session_and_buffer,
    components::{Buffer, BufferName, BufferSlice, BufferView, EditorCtx, Level, Session, Status},
    ex::{ExError, ExRange, ExRangeSolverArgs, solve_exrange},
    misc,
    systems::event::on_buffer_saved,
};

pub fn save_active(
    ctx: &mut EditorCtx,
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    ex_range: ExRange,
) -> Result<(), ExError> {
    let status = &mut ctx.status;
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    save(
        (session, buf_view, buffer),
        status,
        name,
        append,
        only_if_dirty,
        ex_range,
    )?;

    Ok(())
}

pub fn hard_save_active(
    ctx: &mut EditorCtx,
    name: Option<BufferName>,
    append: bool,
    only_if_dirty: bool,
    ex_range: ExRange,
) -> Result<(), ExError> {
    let status = &mut ctx.status;
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    hard_save(
        (session, buf_view, buffer),
        status,
        name,
        append,
        only_if_dirty,
        ex_range,
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
            ExRange::all(),
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
            ExRange::all(),
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
    ex_range: ExRange,
) -> Result<(), ExError> {
    let (session, buf_view, buffer) = session_args;
    let curr_line = buf_view.cursor.row;

    if only_if_dirty && !buffer.is_dirty() {
        return Ok(());
    }

    let range = ex_range.coerce_implicit_to(ExRange::all());
    let line_range = rope_slice(&buffer, curr_line, range)?;
    let buf_slice = BufferSlice::new(buffer, line_range);
    let saves_full_buffer = buf_slice.is_full_buffer();

    match (name.as_ref(), session.buf_name.as_ref()) {
        (None, None) => Err(ExError::NoFileName),
        (None, Some(orig_name)) => {
            // Can't append to myself
            if append {
                return Err(ExError::FileExists);
            }
            // Can't do a partial write to myself
            if !saves_full_buffer {
                return Err(ExError::PartialWrite);
            }
            save_buffer(status, orig_name, append, buf_slice)?;
            buffer.saved();
            Ok(())
        }
        (Some(given_name), None) => {
            if given_name.exists()? {
                return Err(ExError::FileExists);
            }
            save_buffer(status, given_name, append, buf_slice)?;
            if !append && saves_full_buffer {
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
                if !saves_full_buffer {
                    return Err(ExError::PartialWrite);
                }
            } else if given_name.exists()? {
                return Err(ExError::FileExists);
            }
            save_buffer(status, given_name, append, buf_slice)?;
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
    ex_range: ExRange,
) -> Result<(), ExError> {
    let (session, buf_view, buffer) = session_args;
    let curr_line = buf_view.cursor.row;

    if only_if_dirty && !buffer.is_dirty() {
        return Ok(());
    }

    let range = ex_range.coerce_implicit_to(ExRange::all());
    let line_range = rope_slice(&buffer, curr_line, range)?;
    let buf_slice = BufferSlice::new(buffer, line_range);
    let saves_full_buffer = buf_slice.is_full_buffer();

    match (name.as_ref(), session.buf_name.as_ref()) {
        (None, None) => Err(ExError::NoFileName),
        (None, Some(orig_name)) => {
            hard_save_buffer(status, orig_name, append, buf_slice)?;
            if !append && saves_full_buffer {
                buffer.saved();
            }
            Ok(())
        }
        (Some(given_name), _) => {
            hard_save_buffer(status, given_name, append, buf_slice)?;
            if !append && saves_full_buffer {
                if session.buf_name.is_none() {
                    session.buf_name = name;
                }
                buffer.saved();
            }
            Ok(())
        }
    }
}

fn rope_slice(
    buffer: &Buffer,
    curr_line: usize,
    ex_range: ExRange,
) -> Result<Range<usize>, ExError> {
    let args = ExRangeSolverArgs::new(buffer.rope(), buffer.marks());
    let range = solve_exrange(&ex_range, args, curr_line)?;
    if range.is_empty() || range.start > range.end || range.end > buffer.rope().len_lines() {
        Err(ExError::InvalidRange)
    } else {
        Ok(range)
    }
}

fn save_buffer(
    status: &mut Status,
    name: &BufferName,
    append: bool,
    buf_slice: BufferSlice,
) -> Result<(), ExError> {
    let rope_info = misc::io::save(name.file_path.as_path(), append, buf_slice)?;
    on_buffer_saved(status, name, &rope_info);
    Ok(())
}

fn hard_save_buffer(
    status: &mut Status,
    name: &BufferName,
    append: bool,
    buf_slice: BufferSlice,
) -> Result<(), ExError> {
    let rope_info = misc::io::hard_save(name.file_path.as_path(), append, buf_slice)?;
    on_buffer_saved(status, name, &rope_info);
    Ok(())
}
