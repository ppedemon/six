use std::{fmt::Write, format};

use ropey::{Rope, RopeSlice};

use crate::{
    cmd::MotionMode,
    components::{BufferName, Level, Status, YankData},
    misc::time,
    rope,
    systems::immediate,
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

pub fn on_empty_undo_stack(status: &mut Status) {
    status.set_msg(Level::Info, "Already at oldest change");
}

pub fn on_undo(
    status: &mut Status,
    timestamp_ms: u64,
    change_id: u64,
    buf_report: immediate::BufReport,
) {
    let (count, singular, plural) = match buf_report {
        immediate::BufReport::Changed(changes) => (changes, "change", "changes"),
        immediate::BufReport::Added(lines) => (lines, "line added", "lines added"),
        immediate::BufReport::Removed(lines) => (lines, "line deleted", "lines deleted"),
    };
    let change = if count == 1 { singular } else { plural };

    status.set_msg(Level::Info, "");
    write!(status.msg, "{count} {change}; before #{change_id}  ")
        .expect("writing to a String cannot fail");
    time::ppr_timestamp_ms(&mut status.msg, timestamp_ms).expect("writing to a String cannot fail");
}
