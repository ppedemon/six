use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};

use crate::{
    cmd::{Cmd, Kind, Motion, Operator, Scope, Secondary, TextObject},
    components::EditorCtx,
    systems::{input::handler::dispatch, status},
};

enum State {
    Init,
    // Saw: ", we want a reg
    Reg1,
    // We have a reg, follows either reps or op
    Reg2 {
        reg: char,
    },
    // We have a reg and then reps, we need an op
    RegReps {
        reg: char,
        reps: usize,
    },
    // We have reps, follows either reg or op
    Reps {
        reps: usize,
    },
    // Saw ", if follows a reg
    RepsReg1 {
        reps: usize,
    },
    // We have reps and a reg, we need an op
    RepsReg2 {
        reps: usize,
        reg: char,
    },
    // We have an op, must follow motion reps or target
    Operator {
        reps: usize,
        reg: Option<char>,
        op: Operator,
    },
    // Saw motion reps, target must follow
    MotionReps {
        reps: usize,
        reg: Option<char>,
        op: Operator,
        motion_reps: usize,
    },
    // Target was text object, and we have the text object's scope
    TextObject {
        reps: usize,
        reg: Option<char>,
        op: Operator,
        scope: Scope,
    },
}

pub struct NormalInputHandler {
    state: State,
    buffer: String,
}

impl NormalInputHandler {
    pub fn new() -> Self {
        Self {
            state: State::Init,
            buffer: String::with_capacity(256),
        }
    }

    fn reset(&mut self, ctx: &EditorCtx) -> Result<()> {
        status::clear_cmd(ctx)?;
        self.state = State::Init;
        self.buffer.clear();
        Ok(())
    }

    fn done(&mut self, ctx: &EditorCtx, cmd: Cmd) -> Result<()> {
        self.reset(ctx)?;
        dispatch(ctx, cmd)
    }

    pub fn handle_event(&mut self, ctx: &EditorCtx, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) => self.handle_key(ctx, key_event),
            _ => Ok(()),
        }
    }

    fn handle_key(&mut self, ctx: &EditorCtx, event: KeyEvent) -> Result<()> {
        if event.code == KeyCode::Esc {
            self.reset(ctx)?;
            return Ok(());
        }

        self.buffer.extend(event.code.to_string().chars());
        status::set_cmd(ctx, &self.buffer)?;

        let curr_state = std::mem::replace(&mut self.state, State::Init);
        match curr_state {
            State::Init => self.handle_init(ctx, event),
            State::Reps { reps } => self.handle_reps(ctx, event, reps),
            State::RepsReg1 { reps } => self.handle_reps_reg1(ctx, event, reps),
            State::RepsReg2 { reps, reg } => self.handle_reps_reg2(ctx, event, reps, reg),
            State::Reg1 => self.handle_reg1(ctx, event),
            State::Reg2 { reg } => self.handle_reg2(ctx, event, reg),
            State::RegReps { reg, reps } => self.handle_reg_reps(ctx, event, reg, reps),
            State::Operator { reps, reg, op } => self.handle_op(ctx, event, reps, reg, op),
            State::MotionReps {
                reps,
                reg,
                op,
                motion_reps,
            } => self.handle_motion_reps(ctx, event, reps, reg, op, motion_reps),
            State::TextObject {
                reps,
                reg,
                op,
                scope,
            } => self.handle_text_object(ctx, event, reps, reg, op, scope),
        }
    }

    fn handle_init(&mut self, ctx: &EditorCtx, event: KeyEvent) -> Result<()> {
        match (Operator::from(event), as_digit(&event), starts_reg(&event)) {
            (Some(op), _, _) => {
                if op.needs_target() {
                    self.state = State::Operator {
                        reps: 1,
                        reg: None,
                        op,
                    };
                } else {
                    let cmd = Cmd::new(op);
                    self.done(ctx, cmd)?;
                }
            }
            (_, Some(d), _) => self.state = State::Reps { reps: d as usize },
            (_, _, true) => self.state = State::Reg1,
            _ => self.reset(ctx)?,
        }
        Ok(())
    }

    fn handle_reps(&mut self, ctx: &EditorCtx, event: KeyEvent, reps: usize) -> Result<()> {
        match (as_digit(&event), Operator::from(event), starts_reg(&event)) {
            (Some(d), _, _) => {
                let new_reps = reps.saturating_mul(10).saturating_add(d as usize);
                self.state = State::Reps { reps: new_reps };
            }
            (_, Some(op), _) => {
                if op.needs_target() {
                    self.state = State::Operator {
                        reps,
                        reg: None,
                        op,
                    }
                } else {
                    let cmd = Cmd::new(op).reps(reps);
                    self.done(ctx, cmd)?;
                };
            }
            (_, _, true) => self.state = State::RepsReg1 { reps },
            _ => self.reset(ctx)?,
        }
        Ok(())
    }

    fn handle_reps_reg1(&mut self, ctx: &EditorCtx, event: KeyEvent, reps: usize) -> Result<()> {
        match as_reg(&event) {
            Some(reg) => self.state = State::RepsReg2 { reps, reg },
            None => self.reset(ctx)?,
        }
        Ok(())
    }

    fn handle_reps_reg2(
        &mut self,
        ctx: &EditorCtx,
        event: KeyEvent,
        reps: usize,
        reg: char,
    ) -> Result<()> {
        match Operator::from(event) {
            // TODO Fix
            Some(op) => {
                if op.needs_target() {
                    self.state = State::Operator {
                        reps,
                        reg: Some(reg),
                        op,
                    }
                } else {
                    let cmd = Cmd::new(op).reps(reps).reg(Some(reg));
                    self.done(ctx, cmd)?;
                }
            }
            None => self.reset(ctx)?,
        }
        Ok(())
    }

    fn handle_reg1(&mut self, ctx: &EditorCtx, event: KeyEvent) -> Result<()> {
        match as_reg(&event) {
            Some(reg) => self.state = State::Reg2 { reg },
            None => self.reset(ctx)?,
        }
        Ok(())
    }

    fn handle_reg2(&mut self, ctx: &EditorCtx, event: KeyEvent, reg: char) -> Result<()> {
        match (Operator::from(event), as_digit(&event)) {
            (Some(op), _) => {
                if op.needs_target() {
                    self.state = State::Operator {
                        reps: 1,
                        reg: Some(reg),
                        op,
                    }
                } else {
                    let cmd = Cmd::new(op).reg(Some(reg));
                    self.done(ctx, cmd)?;
                };
            }
            (_, Some(d)) => {
                self.state = State::RegReps {
                    reg,
                    reps: d as usize,
                }
            }
            _ => self.reset(ctx)?,
        }
        Ok(())
    }

    fn handle_reg_reps(
        &mut self,
        ctx: &EditorCtx,
        event: KeyEvent,
        reg: char,
        reps: usize,
    ) -> Result<()> {
        match (as_digit(&event), Operator::from(event)) {
            (Some(d), _) => {
                let new_reps = reps.saturating_mul(10).saturating_add(d as usize);
                self.state = State::RegReps {
                    reg,
                    reps: new_reps,
                };
            }
            (_, Some(op)) => {
                if op.needs_target() {
                    self.state = State::Operator {
                        reps,
                        reg: Some(reg),
                        op,
                    }
                } else {
                    // TODO Fix
                    let cmd = Cmd::new(op).reps(reps);
                    self.done(ctx, cmd)?;
                };
            }
            _ => self.reset(ctx)?,
        }
        Ok(())
    }

    fn handle_op(
        &mut self,
        ctx: &EditorCtx,
        event: KeyEvent,
        reps: usize,
        reg: Option<char>,
        op: Operator,
    ) -> Result<()> {
        // The find char operator family must interprect whetever follows as
        // the char to find, so we need to prematurely match on the operator
        match &op {
            Operator::Find(_) => {
                if let Some(c) = as_char(&event) {
                    let cmd = Cmd::new(op).reps(reps).reg(reg).special(Secondary::Char(c));
                    return self.done(ctx, cmd);
                }
                self.reset(ctx)?
            }
            _ => {}
        }

        let (motion, scope, special) = (
            Motion::from(event),
            Scope::from(event),
            Secondary::from(event),
        );

        match (motion, scope, special) {
            (None, None, None) => match as_digit(&event) {
                None => self.reset(ctx)?,
                Some(d) => {
                    self.state = State::MotionReps {
                        reps,
                        reg,
                        op,
                        motion_reps: d as usize,
                    }
                }
            },
            (Some(motion), None, None) => {
                let cmd = Cmd::new(op).reps(reps).reg(reg).motion(motion);
                self.done(ctx, cmd)?;
            }
            (None, Some(scope), None) => {
                self.state = State::TextObject {
                    reps,
                    reg,
                    op,
                    scope,
                }
            }
            (None, None, Some(special)) => {
                let cmd = Cmd::new(op).reps(reps).reg(reg).special(special);
                self.done(ctx, cmd)?;
            }
            _ => self.reset(ctx)?,
        }
        Ok(())
    }

    fn handle_motion_reps(
        &mut self,
        ctx: &EditorCtx,
        event: KeyEvent,
        reps: usize,
        reg: Option<char>,
        op: Operator,
        motion_reps: usize,
    ) -> Result<()> {
        let (d, motion, scope, special) = (
            as_digit(&event),
            Motion::from(event),
            Scope::from(event),
            Secondary::from(event),
        );
        match (d, motion, scope, special) {
            (Some(d), _, _, _) => {
                let new_motion_reps = motion_reps.saturating_mul(10).saturating_add(d as usize);
                self.state = State::MotionReps {
                    reps,
                    reg,
                    op,
                    motion_reps: new_motion_reps,
                };
            }
            (_, Some(motion), _, _) => {
                let cmd = Cmd::new(op)
                    .reps(reps.saturating_mul(motion_reps))
                    .reg(reg)
                    .motion(motion);
                self.done(ctx, cmd)?;
            }
            (_, _, Some(scope), _) => {
                self.state = State::TextObject {
                    reps: reps.saturating_mul(motion_reps),
                    reg,
                    op,
                    scope,
                }
            }
            (_, _, _, Some(special)) => {
                let cmd = Cmd::new(op).reps(reps).reg(reg).special(special);
                self.done(ctx, cmd)?;
            }
            _ => self.reset(ctx)?,
        }
        Ok(())
    }

    fn handle_text_object(
        &mut self,
        ctx: &EditorCtx,
        event: KeyEvent,
        reps: usize,
        reg: Option<char>,
        op: Operator,
        scope: Scope,
    ) -> Result<()> {
        match Kind::from(event) {
            None => self.reset(ctx)?,
            Some(kind) => {
                let text_object = TextObject { scope, kind };
                // TODO Fix
                let cmd = Cmd::new(op).reps(reps).reg(reg).text_object(text_object);
                self.done(ctx, cmd)?;
            }
        }
        Ok(())
    }
}

fn as_digit(event: &KeyEvent) -> Option<u32> {
    if event.modifiers.is_empty() {
        event.code.as_char().and_then(|c| c.to_digit(10))
    } else {
        None
    }
}

fn as_char(event: &KeyEvent) -> Option<char> {
    match event.code {
        KeyCode::Char(c) => Some(c),
        KeyCode::Tab => Some('\t'),
        _ => None,
    }
}

fn starts_reg(event: &KeyEvent) -> bool {
    event.code.as_char().is_some_and(|c| c == '"')
}

fn as_reg(event: &KeyEvent) -> Option<char> {
    event.code.as_char().and_then(|c| match c {
        _ if c.is_ascii_digit() => Some(c),
        _ if c.is_ascii_alphabetic() => Some(c),
        _ if "%#.:/=-_".contains(c) => Some(c),
        _ => None,
    })
}
