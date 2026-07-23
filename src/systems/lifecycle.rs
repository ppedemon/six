use ropey::Rope;
use std::{fs::File, io::BufReader, path::PathBuf};

use crate::{
    components::{Buffer, BufferId, BufferName, BufferView, EditorCtx, Session, SessionId},
    misc::path::norm_filename,
    rope,
    systems::event,
};

pub fn create_editor() -> EditorCtx {
    EditorCtx::new()
}

pub fn should_quit(ctx: &EditorCtx) -> bool {
    ctx.editor.quit
}

// Hard quit, no questions asked
pub fn quit_editor(ctx: &mut EditorCtx) {
    ctx.editor.quit = true;
}

pub fn load_session(ctx: &mut EditorCtx, filename: &str) -> Result<(), std::io::Error> {
    let file_path = norm_filename(filename);
    let buf_name = BufferName::new(filename, file_path.clone());

    let buf_id = create_buffer(ctx, &buf_name)?;
    let buf_view = BufferView::empty();
    let session = Session::new(buf_name, buf_id);
    let session_id = ctx.spawn_session(session, buf_view);
    set_active_session(ctx, session_id);
    Ok(())
}

pub fn create_empty_session(ctx: &mut EditorCtx) {
    let buffer = Buffer::empty();
    let buf_id = ctx.spawn_buffer(buffer);
    let buf_view = BufferView::empty();
    let session = Session::empty(buf_id);
    let session_id = ctx.spawn_session(session, buf_view);
    set_active_session(ctx, session_id);
}

fn create_buffer(ctx: &mut EditorCtx, buf_name: &BufferName) -> Result<BufferId, std::io::Error> {
    let file_path = &buf_name.file_path;
    let buf_id = match find_buffer(ctx, file_path) {
        Some(buf_id) => buf_id,
        None => {
            let buffer = if file_path.exists() {
                let file = File::open(file_path)?;
                let reader = BufReader::new(file);
                let mut rope = Rope::from_reader(reader)?;
                rope::norm(&mut rope);
                Buffer::new(rope)
            } else {
                Buffer::empty()
            };

            event::on_buffer_loaded(&mut ctx.status, buf_name, &buffer.rope());
            ctx.spawn_buffer(buffer)
        }
    };

    Ok(buf_id)
}

fn find_buffer(ctx: &EditorCtx, file_path: &PathBuf) -> Option<BufferId> {
    for (session, _) in ctx.sessions.values() {
        if session
            .buf_name
            .as_ref()
            .is_some_and(|name| name.file_path.as_path() == file_path.as_path())
        {
            return Some(session.buf_id);
        }
    }
    None
}

fn set_active_session(ctx: &mut EditorCtx, session_id: SessionId) {
    ctx.editor.session_id = session_id;
}
