use crate::{
    cmd::Cmd,
    components::EditorCtx,
    systems::{event, nav::interpret},
};

pub fn yank(ctx: &mut EditorCtx, cmd: Cmd) {
    let Ok((yank_data, reg_data)) = interpret(ctx, cmd) else {
        // TODO Maybe show an error here?
        return;
    };

    event::on_yank(&mut ctx.status, yank_data);
    ctx.registers.record_yank(cmd.reg, reg_data);
    ctx.repbuf.save_last_yank(yank_data);
}
