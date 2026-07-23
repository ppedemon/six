use crate::{
    cmd::{Cmd, Motion},
    components::{Buffer, EditorCtx, Level},
    ex::{BuiltIn, ExError, ExRange},
    systems::{
        ex::{
            args::{validate_no_args, validate_opt_append_filename},
            fs,
        },
        lifecycle,
        nav::{NavArgs, handle_nav},
    },
};

pub fn exec_builtin(
    ctx: &mut EditorCtx,
    range: ExRange,
    builtin: BuiltIn,
    args: &str,
) -> Result<(), ExError> {
    match builtin {
        BuiltIn::Quit("q!") => {
            validate_no_args(args)?;
            lifecycle::quit_editor(ctx);
        }
        BuiltIn::Quit("q") => {
            validate_no_args(args)?;
            if is_editor_dirty(ctx) {
                return Err(ExError::UnsavedChanges);
            } else {
                lifecycle::quit_editor(ctx);
            }
        }
        BuiltIn::Quit("wq!") => {
            validate_no_args(args)?;
            fs::hard_save_active(ctx, None, false, false, range)?;
            lifecycle::quit_editor(ctx);
        }
        BuiltIn::Quit("wq") => {
            validate_no_args(args)?;
            fs::save_active(ctx, None, false, false, range)?;
            lifecycle::quit_editor(ctx);
        }
        BuiltIn::Quit("x!") => {
            validate_no_args(args)?;
            fs::hard_save_active(ctx, None, false, true, range)?;
            lifecycle::quit_editor(ctx);
        }
        BuiltIn::Quit("x") => {
            validate_no_args(args)?;
            fs::save_active(ctx, None, false, true, range)?;
            lifecycle::quit_editor(ctx);
        }
        BuiltIn::Write("w!") => {
            let (append, name) = validate_opt_append_filename(args)?;
            fs::hard_save_active(ctx, name, append, false, range)?;
        }
        BuiltIn::Write("w") => {
            let (append, name) = validate_opt_append_filename(args)?;
            fs::save_active(ctx, name, append, false, range)?;
        }
        BuiltIn::GotoLine(line) => {
            let motion = Motion::GotoLine(line);
            let cmd = Cmd::new(motion.into());
            handle_nav(ctx, NavArgs::new(motion, cmd));
            ctx.status.set_msg(Level::Info, &format!(":{}", line));
        }
        _ => unreachable!("Unimplemented builtin: {builtin:?}"),
    }

    Ok(())
}

fn is_editor_dirty(ctx: &EditorCtx) -> bool {
    ctx.buffers.values().any(Buffer::is_dirty)
}
