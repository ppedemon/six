use crate::{
    components::{Buffer, EditorCtx},
    ex::{BuiltIn, ExError, ExRange},
    systems::{
        ex::{
            args::{validate_no_args, validate_opt_filename},
            fs,
        },
        lifecycle,
    },
};

pub fn exec_builtin(
    ctx: &mut EditorCtx,
    range: ExRange,
    builtin: BuiltIn,
    args: &str,
) -> Result<(), ExError> {
    match builtin {
        BuiltIn::Quit { bang } => quit(ctx, range, args, bang)?,
        BuiltIn::Exit { bang } => exit(ctx, range, args, bang)?,
        BuiltIn::Write { bang, append, quit } => write(ctx, range, args, bang, append, quit)?,
    }
    Ok(())
}

fn quit(ctx: &mut EditorCtx, range: ExRange, args: &str, bang: bool) -> Result<(), ExError> {
    validate_range(&range, |p| matches!(p, ExRange::Implicit))?;
    validate_no_args(args)?;
    if bang {
        lifecycle::quit_editor(ctx);
    } else {
        soft_quit(ctx)?;
    }
    Ok(())
}

fn exit(ctx: &mut EditorCtx, range: ExRange, args: &str, bang: bool) -> Result<(), ExError> {
    let name = validate_opt_filename(args)?;
    if bang {
        fs::hard_save_active(ctx, name, false, true, range)?;
    } else {
        fs::save_active(ctx, name, false, true, range)?;
    }
    lifecycle::quit_editor(ctx);
    Ok(())
}

fn write(
    ctx: &mut EditorCtx,
    range: ExRange,
    args: &str,
    bang: bool,
    append: bool,
    quit: bool,
) -> Result<(), ExError> {
    let name = validate_opt_filename(args)?;
    if bang {
        fs::hard_save_active(ctx, name, append, false, range)?;
    } else {
        fs::save_active(ctx, name, append, false, range)?;
    }
    if quit {
        lifecycle::quit_editor(ctx);
    }
    Ok(())
}

fn soft_quit(ctx: &mut EditorCtx) -> Result<(), ExError> {
    if ctx.buffers.values().any(Buffer::is_dirty) {
        Err(ExError::UnsavedChanges)
    } else {
        lifecycle::quit_editor(ctx);
        Ok(())
    }
}

fn validate_range(range: &ExRange, pred: fn(&ExRange) -> bool) -> Result<(), ExError> {
    if pred(range) {
        Ok(())
    } else {
        Err(ExError::UnsupportedRange)
    }
}
