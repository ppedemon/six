use crate::{
    active_session_and_buffer,
    cmd::{Arg, Cmd},
    components::{EditorCtx, MutBuffer, RegisterData},
    systems::{
        commons::cursor_to_char_idx,
        event,
        immediate::yank::motion_yank,
        insert::Damage,
        nav::{self, NormalNav, utils::ensure_cursor_inside_line},
    },
};

pub fn delete(ctx: &mut EditorCtx, cmd: Cmd) -> Damage {
    match cmd.arg {
        Arg::Motion { reps, mode, motion } => {
            let cmd_reps = cmd.reps.unwrap_or(1);
            let arg_reps = reps.unwrap_or(1);
            match motion_yank(ctx, motion, cmd_reps, arg_reps, mode) {
                None => Damage::Intact,
                Some(reg_data) => {
                    let damage = delete_data(ctx, &reg_data);
                    notity_delete(ctx, &reg_data);
                    ctx.registers.record_delete(cmd.reg, reg_data);
                    damage
                }
            }
        }
        // TODO Implement text-object movement and deletion
        Arg::TextObject { .. } => Damage::Intact,
        Arg::None => Damage::Intact,
    }
}

fn delete_data(ctx: &mut EditorCtx, reg_data: &RegisterData) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    match reg_data {
        RegisterData::Char { data } => delete_charwise(ctx, data),
        RegisterData::Line { data } => delete_linewise(ctx, data),
        RegisterData::Block { data } => Damage::Intact,
    }
}

fn delete_charwise(ctx: &mut EditorCtx, data: &str) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let cursor = buf_view.cursor;
    let start_idx = cursor_to_char_idx(&ctx.config, buf_view, buffer.rope());
    let len_chars = data.chars().count();

    buffer.edit().remove(start_idx..start_idx + len_chars);

    let damage = if data.lines().count() <= 1 {
        buf_view
            .display_buf
            .patch_range(&ctx.config, buffer.rope(), cursor.row..cursor.row + 1);
        Damage::Line(cursor.row)
    } else {
        buf_view.display_buf.destroy_from(cursor.row);
        Damage::From(cursor.row)
    };

    ensure_cursor_inside_line(ctx);
    damage
}

fn delete_linewise(ctx: &mut EditorCtx, data: &str) -> Damage {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);

    let cursor = buf_view.cursor;
    let start_idx = buffer.rope().line_to_char(cursor.row);
    let data = &data[0..data.len().min(buffer.rope().len_chars() - start_idx)];
    let len_chars = data.chars().count();

    buffer.edit().remove(start_idx..start_idx + len_chars);
    buf_view.display_buf.destroy_from(cursor.row);

    nav::line_first_non_blank::<NormalNav>(&ctx.config, buffer.rope(), buf_view);
    Damage::From(cursor.row)
}

fn notity_delete(ctx: &mut EditorCtx, reg_data: &RegisterData) {
    let (_, _, buffer) = active_session_and_buffer!(ctx);
    event::on_delete(&mut ctx.status, reg_data, buffer.rope().len_chars() == 0);
}
