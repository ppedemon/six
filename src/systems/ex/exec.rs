use crate::{
    active_session_and_buffer,
    cmd::{Cmd, Motion, Operator},
    components::{Direction, EditorCtx, ExState, Level},
    ex::{ExCmd, ExError, ExRange, ExRangeSolverArgs, parse_cmd_line, solve_exrange},
    systems::{ex::builtin::exec_builtin, input::dispatch_cmd, mode::enter_normal},
};

pub fn handle_ex_state(ctx: &mut EditorCtx) {
    let ex_state = std::mem::replace(&mut ctx.ex_session.state, ExState::Idle);

    match ex_state {
        ExState::Idle => {}
        ExState::Cancel => enter_normal(ctx),
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
    if let Some(i) = s.find(|c: char| !c.is_ascii_whitespace() && c != ':') {
        let cmd_line = &s[i..];
        if cmd_line.len() > 0 {
            exec_cmd_line(ctx, cmd_line)?;
        }
    }
    Ok(())
}

fn exec_search(ctx: &mut EditorCtx, cmd_line: &str, direction: Direction) -> Result<(), ExError> {
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
    let range = solve_exrange(ex_range.clone(), args, buf_view.cursor.row)?;

    dispatch_cmd(ctx, Cmd::new(Operator::Move(Motion::GotoLine(range.end))));
    ctx.status.set_msg(Level::Info, &format!(":{}", ex_range));

    Ok(())
}

// TODO Implement
fn exec_shell(_ctx: &EditorCtx, _range: ExRange, _raw_cmd: &str) -> Result<(), ExError> {
    Ok(())
}
