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
        RegisterData::Char { rope } => {
            if rope.len_lines() > 2 {
                let msg = format!(
                    "{} {} yanked",
                    rope.len_lines(),
                    if rope.len_chars() == 1 {
                        "line"
                    } else {
                        "lines"
                    }
                );
                status.set_msg(Level::Info, &msg);
            }
        }
        RegisterData::Line { rope } => {
            let lines = rope.lines().filter(|line| line.len_chars() > 0).count();
            if lines > 2 {
                let msg = format!(
                    "{} {} yanked",
                    lines,
                    if lines == 1 { "line" } else { "lines" }
                );
                status.set_msg(Level::Info, &msg);
            }
        }
        RegisterData::Block { rope } => {
            if rope.len_lines() > 2 {
                let line_len = |slice: RopeSlice<'_>| {
                    let len = slice.len_chars();
                    if len > 0 && slice.char(len - 1) == '\n' {
                        len - 1
                    } else {
                        len
                    }
                };

                let mut lens = rope.lines().map(line_len);
                let perfect = match lens.next() {
                    Some(n) => lens.all(|x| x == n),
                    None => true,
                };

                let rows = rope.len_lines();
                let msg = if perfect {
                    let cols = if rows > 0 { line_len(rope.line(0)) } else { 0 };
                    format!("{rows}x{cols} block yanked",)
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
        RegisterData::Line { rope } => {
            let lines = rope.len_lines().saturating_sub(1).max(1) * reps;
            if lines > 2 {
                let msg = format!(
                    "{lines} {} pasted",
                    if lines == 1 { "line" } else { "lines" }
                );
                status.set_msg(Level::Info, &msg);
            }
        }
        _ => {}
    }
}
