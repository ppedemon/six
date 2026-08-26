use crate::{
    active_session_and_buffer,
    components::{Buffer, EditorCtx, MutBuffer},
    systems::{commons::char_idx_to_coords, insert::Damage},
};

// Join N lines (J command)
pub fn join(ctx: &mut EditorCtx, reps: usize) -> Damage {
    let (session, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let row = buf_view.cursor.row;
    if row + 1 == buffer.rope().len_lines() {
        return Damage::Intact;
    }

    let mut reps = reps.saturating_sub(1);
    let mut cursor_idx;

    loop {
        cursor_idx = join_single(buffer, row);

        reps = reps.saturating_sub(1);
        if reps == 0 || row + 1 == buffer.rope().len_lines() {
            break;
        }
    }

    buf_view.display_buf.destroy_from(row);
    buf_view.cursor = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, cursor_idx);

    Damage::From(row)
}

fn join_single(buffer: &mut Buffer, row: usize) -> usize {
    let boundary = buffer.rope().line_to_char(row + 1);

    let mut start_idx = boundary - 1;
    if start_idx > 0
        && buffer.rope().char(start_idx) == '\n'
        && buffer.rope().char(start_idx - 1) == '\r'
    {
        start_idx -= 1;
    }

    let end_idx = buffer
        .rope()
        .chars_at(boundary)
        .position(|c| !c.is_whitespace() || c == '\n' || c == '\r')
        .map(|pos| boundary + pos)
        .unwrap_or(buffer.rope().len_chars());

    buffer.edit().remove(start_idx..end_idx);

    if start_idx > 0
        && buffer.rope().char(start_idx) != '\r'
        && buffer.rope().char(start_idx) != '\n'
    {
        match buffer.rope().char(start_idx - 1) {
            c if c.is_whitespace() => {}
            c if c == '.' || c == '?' || c == '!' => buffer.edit().insert(start_idx, "  "),
            _ => buffer.edit().insert_char(start_idx, ' '),
        }
    }

    let cursor_idx =
        if buffer.rope().char(start_idx) == '\r' || buffer.rope().char(start_idx) == '\n' {
            start_idx.saturating_sub(1)
        } else {
            start_idx
        };

    cursor_idx
}
