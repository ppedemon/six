use std::format;

use ropey::{Rope, RopeSlice};

use crate::{
    cmd::MotionMode,
    components::{BufferName, Level, Status, YankData},
    rope,
};

pub fn on_buffer_loaded(status: &mut Status, name: &BufferName, rope: &Rope) {
    let msg = if name.file_path.exists() {
        let rope_info = rope::info(rope);
        let lines = rope_info.num_lines;
        let bytes = rope_info.num_bytes;
        &format!("\"{}\" - {}L, {}B", name.orig_name, lines, bytes)
    } else {
        &format!("\"{}\" - [New]", name.orig_name)
    };
    status.set_msg(Level::Info, msg);
}

pub fn on_buffer_saved(status: &mut Status, name: &BufferName, rope: RopeSlice<'_>) {
    let msg = {
        let rope_info = rope::info_slice(rope);
        let lines = rope_info.num_lines;
        let bytes = rope_info.num_bytes;
        &format!("\"{}\" - {}L, {}B written", name.orig_name, lines, bytes)
    };
    status.set_msg(Level::Info, msg);
}

pub fn on_yank(status: &mut Status, yank_data: YankData) {
    let num_lines = yank_data.num_lines();
    if num_lines > 2 {
        let msg = if yank_data.mode == MotionMode::Blockwise {
            format!("Block of {num_lines} lines yanked")
        } else {
            format!("{num_lines} lines yanked")
        };
        status.set_msg(Level::Info, &msg);
    } else {
        status.clear_msg();
    }
}

pub fn on_paste(status: &mut Status, num_lines: usize) {
    if num_lines > 2 {
        let msg = format!("{num_lines} pasted");
        status.set_msg(Level::Info, &msg);
    } else {
        status.clear_msg();
    }
}

pub fn on_delete(status: &mut Status, num_lines: usize, is_empty: bool) {
    if is_empty {
        status.set_msg(Level::Info, "-- No lines in buffer --");
    } else if num_lines > 2 {
        let msg = format!("{num_lines} lines deleted");
        status.set_msg(Level::Info, &msg);
    } else {
        status.clear_msg();
    }
}
