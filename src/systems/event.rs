use anyhow::Result;
use ropey::RopeSlice;

use crate::{
    components::{Buffer, BufferName, EditorCtx, Level},
    rope,
    systems::status,
};

pub fn on_buffer_loaded(ctx: &EditorCtx, buffer: &Buffer) -> Result<()> {
    if let Some(name) = buffer.name.as_ref() {
        let msg = if name.file_path.exists() {
            let rope_info = rope::info(&buffer.rope);
            let lines = rope_info.num_lines;
            let bytes = rope_info.num_bytes;
            &format!("\"{}\" - {}L, {}B", name.orig_name, lines, bytes)
        } else {
            &format!("\"{}\" - [New]", name.orig_name)
        };
        status::set_msg(ctx, Level::Info, msg)?;
    }

    Ok(())
}

pub fn on_buffer_saved(ctx: &EditorCtx, name: &BufferName, rope: RopeSlice<'_>) -> Result<()> {
    let msg = {
        let rope_info = rope::info_slice(rope);
        let lines = rope_info.num_lines;
        let bytes = rope_info.num_bytes;
        &format!("\"{}\" - {}L, {}B written", name.orig_name, lines, bytes)
    };
    status::set_msg(ctx, Level::Info, msg)?;
    Ok(())
}
