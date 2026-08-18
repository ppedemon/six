use std::format;

use ropey::{Rope, RopeSlice};

use crate::{
    components::{BufferName, Level, RegisterData, Status},
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

pub fn on_yank(status: &mut Status, reg_data: &RegisterData) {
    match reg_data {
        RegisterData::Char { data } | RegisterData::Line { data } => {
            let lines = data.lines().count();
            if lines > 2 {
                let msg = format!("{lines} lines yanked");
                status.set_msg(Level::Info, &msg);
            }
        }
        RegisterData::Block { data } => {
            let rows = data.lines().count();
            if rows > 2 {
                let mut lens = data.lines().map(&str::len);
                let perfect = match lens.next() {
                    Some(n) => lens.all(|x| x == n),
                    None => true,
                };

                let msg = if perfect {
                    let cols = data.lines().next().map_or(0, &str::len);
                    format!("{rows}x{cols} block yanked")
                } else {
                    format!("Block of {rows} rows yanked")
                };
                status.set_msg(Level::Info, &msg);
            }
        }
    };
}

pub fn on_paste(status: &mut Status, reg_data: &RegisterData, reps: usize) {
    match reg_data {
        RegisterData::Char { data } => {
            let lines = data.lines().count();
            let pasted_lines = if lines == 1 { lines } else { lines * reps };
            if pasted_lines > 2 {
                let msg = format!("{pasted_lines} lines pasted");
                status.set_msg(Level::Info, &msg);
            }
        }
        RegisterData::Line { data } => {
            let pasted_lines = data.lines().count() * reps;
            if pasted_lines > 2 {
                let msg = format!("{pasted_lines} lines pasted");
                status.set_msg(Level::Info, &msg);
            }
        }
        RegisterData::Block { data } => {}
    }
}
