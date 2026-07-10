use crate::{
    cmd::{Motion, Arg},
    components::{Buffer, EditorCtx, Level},
    ex::{BuiltIn, ExError, ExRange},
    systems::{
        ex::{
            args::{validate_no_args, validate_opt_append_filename},
            fs,
        },
        lifecycle,
        nav::{NavArgs, handle_nav},
        status,
    },
};

pub fn exec_builtin(
    ctx: &EditorCtx,
    range: ExRange,
    builtin: BuiltIn,
    args: &str,
) -> Result<(), ExError> {
    match builtin {
        BuiltIn::Quit("q!") => {
            validate_no_args(args)?;
            adapt(lifecycle::quit_editor(ctx))
        }
        BuiltIn::Quit("q") => {
            validate_no_args(args)?;
            if is_editor_dirty(ctx) {
                Err(ExError::UnsavedChanges)
            } else {
                adapt(lifecycle::quit_editor(ctx))
            }
        }
        BuiltIn::Quit("wq!") => {
            validate_no_args(args)?;
            fs::hard_save_active(ctx, None, false, false, range)?;
            adapt(lifecycle::quit_editor(ctx))
        }
        BuiltIn::Quit("wq") => {
            validate_no_args(args)?;
            fs::save_active(ctx, None, false, false, range)?;
            adapt(lifecycle::quit_editor(ctx))
        }
        BuiltIn::Quit("x!") => {
            validate_no_args(args)?;
            fs::hard_save_active(ctx, None, false, true, range)?;
            adapt(lifecycle::quit_editor(ctx))
        }
        BuiltIn::Quit("x") => {
            validate_no_args(args)?;
            fs::save_active(ctx, None, false, true, range)?;
            adapt(lifecycle::quit_editor(ctx))
        }
        BuiltIn::Write("w!") => {
            let (append, name) = validate_opt_append_filename(args)?;
            fs::hard_save_active(ctx, name, append, false, range)
        }
        BuiltIn::Write("w") => {
            let (append, name) = validate_opt_append_filename(args)?;
            fs::save_active(ctx, name, append, false, range)
        }
        BuiltIn::GotoLine(line) => {
            adapt(handle_nav(
                ctx,
                NavArgs::new(Motion::BigGotoLine, Some(line), Arg::None),
            ))?;
            adapt(status::set_msg(ctx, Level::Info, &format!(":{}", line)))
        }
        _ => unreachable!("Unimplemented builtin: {builtin:?}"),
    }
}

fn is_editor_dirty(ctx: &EditorCtx) -> bool {
    ctx.world
        .query::<&Buffer>()
        .iter()
        .any(|buffer| buffer.dirty)
}

fn adapt(action: Result<(), anyhow::Error>) -> Result<(), ExError> {
    action.map_err(|source| ExError::InternalError { source })
}
