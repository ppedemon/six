use crate::{
    components::{EditorCtx, ExSession, ExState, Level},
    ex::{ExCmd, ExError, ExRange, parse_cmd_line},
    systems::{ex::builtin::exec_builtin, status, sys::enter_normal},
};

pub fn handle_ex_state(ctx: &EditorCtx) {
    let ex_state = {
        let mut ex_session = ctx.world.get::<&mut ExSession>(ctx.ex_session_id).unwrap();
        std::mem::replace(&mut ex_session.state, ExState::Idle)
    };

    match ex_state {
        ExState::Idle => {}
        ExState::Cancel => {
            enter_normal(ctx).unwrap();
        }
        ExState::Submit(s) => {
            enter_normal(ctx).unwrap();
            if let Some((head, cmd_line)) = split(&s)
                && !cmd_line.is_empty()
            {
                exec(ctx, head, cmd_line).unwrap_or_else(|err| {
                    status::set_msg(ctx, Level::Error, &err.to_string()).unwrap();
                });
            }
        }
    }
}

fn exec(ctx: &EditorCtx, head: char, cmd_line: &str) -> Result<(), ExError> {
    match head {
        ':' => {
            let ex_cmds = parse_cmd_line(cmd_line.as_ref())?;
            for cmd in ex_cmds {
                let result = match cmd {
                    ExCmd::BuiltIn {
                        range,
                        builtin,
                        args,
                    } => exec_builtin(ctx, range, builtin, args),
                    ExCmd::Shell { range, raw_cmd } => exec_shell(ctx, range, raw_cmd),
                };
                if result.is_err() {
                    return result;
                }
            }
            Ok(())
        }
        // TODO Implement forward and backward search
        '/' => Ok(()),
        '?' => Ok(()),
        _ => Ok(()),
    }
}

// TODO Implement
fn exec_shell(_ctx: &EditorCtx, _range: ExRange, _raw_cmd: &str) -> Result<(), ExError> {
    Ok(())
}

fn split(s: &str) -> Option<(char, &str)> {
    let mut chars = s.chars();
    Some((chars.next()?, chars.as_str()))
}
