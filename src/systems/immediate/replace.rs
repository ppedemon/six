use crate::{
    active_session_and_buffer,
    components::{Coords, EditorCtx, MutBuffer},
    systems::{
        commons::{char_idx_to_coords, coords_to_char_idx, curr_line, cursor_to_char_idx},
        insert::Damage,
    },
};

pub fn replace(ctx: &mut EditorCtx, c: char, reps: usize) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let cursor = buf_view.cursor;
    let line = curr_line(&ctx.config, buffer.rope(), buf_view);

    let mut n = 0;
    let mut g = line.grapheme_at(cursor.col);
    while n + 1 < reps
        && let Some((_, span)) = g
    {
        n += 1;
        g = line.grapheme_at(span.end);
    }

    if let Some((_, span)) = g {
        let end_coords = Coords::new(cursor.row, span.end);
        let start_idx = cursor_to_char_idx(&ctx.config, buf_view, buffer.rope());
        let end_idx = coords_to_char_idx(&ctx.config, buffer.rope(), buf_view, end_coords);

        buffer.edit().remove(start_idx..end_idx);
        for _ in 0..reps {
            buffer.edit().insert_char(start_idx, c);
        }

        buf_view
            .display_buf
            .patch_range(&ctx.config, buffer.rope(), cursor.row..cursor.row + 1);

        let cursor_idx = start_idx + reps - 1;
        let cursor = char_idx_to_coords(&ctx.config, buffer.rope(), buf_view, cursor_idx);
        buf_view.cursor = cursor;
        buf_view.target_col = cursor.col;

        Damage::Line(cursor.row)
    } else {
        Damage::Intact
    }
}
