use crate::{
    active_session_and_buffer,
    cmd::{Cmd, Motion, Operator},
    components::{Direction, EditorCtx, ExState, Level},
    ex::{ExCmd, ExError, ExRange, ExRangeSolverArgs, parse_cmd_line, parse_search, solve_exrange},
    systems::{
        ex::builtin::exec_builtin, input::dispatch_cmd, insert::clear_ex, mode::enter_normal,
    },
};

pub fn handle_ex_state(ctx: &mut EditorCtx) {
    let ex_state = std::mem::replace(&mut ctx.ex_session.state, ExState::Idle);

    match ex_state {
        ExState::Idle => {}
        ExState::Cancel => {
            clear_ex(ctx);
            enter_normal(ctx);
        }
        ExState::Submit(s) => {
            enter_normal(ctx);
            if let Some(head) = s.chars().next() {
                let result = match head {
                    ':' => exec_ex(ctx, &s),
                    '/' => exec_search(ctx, &s, Direction::Forward),
                    '?' => exec_search(ctx, &s, Direction::Backward),
                    _ => Ok(()),
                };
                result.unwrap_or_else(|err| {
                    ctx.status.set_msg(Level::Error, &err.to_string());
                });
            }
        }
    }
}

fn exec_ex(ctx: &mut EditorCtx, s: &str) -> Result<(), ExError> {
    let mut no_cmd = true;

    if let Some(i) = s.find(|c: char| !c.is_ascii_whitespace() && c != ':') {
        let cmd_line = &s[i..];
        if cmd_line.len() > 0 {
            exec_cmd_line(ctx, cmd_line)?;
            no_cmd = false;
        }
    }

    if no_cmd {
        clear_ex(ctx);
    }

    Ok(())
}

fn exec_search(ctx: &mut EditorCtx, cmd_line: &str, direction: Direction) -> Result<(), ExError> {
    let p = parse_search(cmd_line)?;

    #[cfg(feature = "dev")]
    tracing::debug!("Result = {p:?}");

    let new_motion = match direction {
        Direction::Forward => Motion::ImmSearchForward,
        Direction::Backward => Motion::ImmSearchBackward,
    };

    ctx.last_search.save_pat_search(p);

    let cmd = ctx.last_search.take_cmd();
    let patched_cmd = cmd.patch_motion(new_motion);
    dispatch_cmd(ctx, patched_cmd);

    Ok(())
}

fn exec_cmd_line(ctx: &mut EditorCtx, cmd_line: &str) -> Result<(), ExError> {
    let ex_cmds = parse_cmd_line(cmd_line.as_ref())?;
    for cmd in ex_cmds {
        let result = match cmd {
            ExCmd::BuiltIn {
                range,
                builtin,
                args,
            } => exec_builtin(ctx, range, builtin, args),
            ExCmd::Shell { range, raw_cmd } => exec_shell(ctx, range, raw_cmd),
            ExCmd::ExRange(range) => exec_range(ctx, range),
        };
        if result.is_err() {
            return result;
        }
    }
    Ok(())
}

fn exec_range(ctx: &mut EditorCtx, ex_range: ExRange) -> Result<(), ExError> {
    let (_, buf_view, buffer) = active_session_and_buffer!(mut ctx);
    let args = ExRangeSolverArgs::new(buffer.rope(), buffer.marks());
    let range = solve_exrange(&ex_range, args, buf_view.cursor.row)?;

    dispatch_cmd(ctx, Cmd::new(Operator::Move(Motion::GotoLine(range.end))));
    Ok(())
}

// TODO Implement
fn exec_shell(_ctx: &EditorCtx, _range: ExRange, _raw_cmd: &str) -> Result<(), ExError> {
    Ok(())
}
