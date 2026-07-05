use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::{Result, anyhow};
use hecs::{Entity, World};
use ropey::Rope;

use crate::{
    components::{
        Buffer, BufferName, BufferView, Config, EditorCtx, EditorState, ExSession, Session, Status,
    },
    misc::path::norm_filename,
    rope,
    systems::{
        event,
        nav::{NormalNav, move_to_first_non_blank},
    },
};

pub fn create_editor(world: &mut World) -> EditorCtx<'_> {
    let config_id = world.spawn((Config::default(),));
    let editor_id = world.spawn((EditorState::new(),));
    let ex_session_id = world.spawn((ExSession::new(), BufferView::empty()));
    let status_id = world.spawn((Status::new(),));

    EditorCtx {
        world,
        config_id,
        editor_id,
        ex_session_id,
        status_id,
    }
}

pub fn should_quit(ctx: &EditorCtx) -> Result<bool> {
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;
    Ok(editor.quit)
}

// Hard quit, no questions asked
pub fn quit_editor(ctx: &EditorCtx) -> Result<()> {
    let mut editor_state = ctx.world.get::<&mut EditorState>(ctx.editor_id)?;
    editor_state.quit = true;
    Ok(())
}

pub fn load_session(ctx: &mut EditorCtx, filename: &str) -> Result<()> {
    let buf_id = create_buffer(ctx, filename)?;
    let buf_view = BufferView::empty();
    let session = Session::new(buf_id);
    let session_id = ctx.world.spawn((session, buf_view));
    set_active_session(ctx, session_id)
}

pub fn create_empty_session(ctx: &mut EditorCtx) -> Result<()> {
    let buffer = Buffer::empty();
    let buf_id = ctx.world.spawn((buffer,));
    let buf_view = BufferView::empty();
    let session = Session::new(buf_id);
    let session_id = ctx.world.spawn((session, buf_view));
    set_active_session(ctx, session_id)
}

fn create_buffer(ctx: &mut EditorCtx, filename: &str) -> Result<Entity> {
    let file_path = norm_filename(filename);

    if file_path.as_os_str().is_empty() {
        return Err(anyhow!("Invalid path: {}", filename));
    }

    let buf_id = match find_buffer(ctx.world, &file_path) {
        Some(buf_id) => buf_id,
        None => {
            let name = BufferName::new(filename, file_path.clone());

            let buffer = if file_path.exists() {
                let file = File::open(&filename)?;
                let reader = BufReader::new(file);
                let mut rope = Rope::from_reader(reader)?;
                rope::norm(&mut rope);
                Buffer::new(name, rope)
            } else {
                Buffer::new(name, Rope::new())
            };

            event::on_buffer_loaded(ctx, &buffer)?;
            ctx.world.spawn((buffer,))
        }
    };

    Ok(buf_id)
}

fn find_buffer(world: &World, file_path: &PathBuf) -> Option<Entity> {
    for (buf_id, buffer) in world.query::<(Entity, &Buffer)>().iter() {
        if buffer
            .name
            .as_ref()
            .is_some_and(|name| name.file_path.as_path() == file_path.as_path())
        {
            return Some(buf_id);
        }
    }
    None
}

fn set_active_session(ctx: &EditorCtx, session_id: Entity) -> Result<()> {
    let mut editor_state = ctx.world.get::<&mut EditorState>(ctx.editor_id)?;
    editor_state.session_id = session_id;
    Ok(())
}

// This should be easy, but moving the cursor to the active document's first non-blank
// character can force arbitrary scrolling, so we have to go through the navigation 
// machinery, fetching all the parameters it requires. Oh well...
pub fn adjust_initial_coords(ctx: &EditorCtx) -> Result<()> {
    let config = ctx.world.get::<&Config>(ctx.config_id)?;
    let editor = ctx.world.get::<&EditorState>(ctx.editor_id)?;

    let mut q_session = ctx
        .world
        .query_one::<(&Session, &mut BufferView)>(editor.session_id);
    let (session, buf_view) = q_session.get()?;

    let buffer = ctx.world.get::<&Buffer>(session.buf_id)?;
    move_to_first_non_blank::<NormalNav>(&config, &buffer.rope, buf_view);
    Ok(())
}
