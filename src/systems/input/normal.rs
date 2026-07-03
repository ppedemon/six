use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};

use crate::{
    components::EditorCtx,
    normal::{
        NormalCmd, ExMode, InsertPoint, Kind, Motion, Operator, Scope, Secondary, SysOp, TextObject,
    },
    systems::{input::handler::dispatch, status},
};

enum State {
    Init,
    CmdReps {
        reps: usize,
    },
    Operator {
        reps: usize,
        op: Operator,
    },
    MotionReps {
        reps: usize,
        op: Operator,
        motion_reps: usize,
    },
    TextObject {
        reps: usize,
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

    fn done(&mut self, ctx: &EditorCtx, cmd: NormalCmd) -> Result<()> {
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
            State::CmdReps { reps } => self.handle_cmd_reps(ctx, event, reps),
            State::Operator { reps, op } => self.handle_op(ctx, event, reps, op),
            State::MotionReps {
                reps,
                op,
                motion_reps,
            } => self.handle_motion_reps(ctx, event, reps, op, motion_reps),
            State::TextObject { reps, op, scope } => {
                self.handle_text_object(ctx, event, reps, op, scope)
            }
        }
    }

    fn handle_init(&mut self, ctx: &EditorCtx, event: KeyEvent) -> Result<()> {
        match operator(event) {
            None => match as_digit(&event) {
                None => self.reset(ctx)?,
                Some(d) => self.state = State::CmdReps { reps: d as usize },
            },
            Some(op) => {
                if op.needs_target() {
                    self.state = State::Operator { reps: 1, op };
                } else {
                    let cmd = NormalCmd::new(op);
                    self.done(ctx, cmd)?;
                }
            }
        }
        Ok(())
    }

    fn handle_cmd_reps(&mut self, ctx: &EditorCtx, event: KeyEvent, reps: usize) -> Result<()> {
        match operator(event) {
            None => match as_digit(&event) {
                None => self.reset(ctx)?,
                Some(d) => {
                    let new_reps = reps.saturating_mul(10).saturating_add(d as usize);
                    self.state = State::CmdReps { reps: new_reps };
                }
            },
            Some(op) => {
                if op.needs_target() {
                    self.state = State::Operator { reps, op }
                } else {
                    let cmd = NormalCmd::new(op).reps(reps);
                    self.done(ctx, cmd)?;
                };
            }
        }
        Ok(())
    }

    fn handle_op(
        &mut self,
        ctx: &EditorCtx,
        event: KeyEvent,
        reps: usize,
        op: Operator,
    ) -> Result<()> {
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
                        op,
                        motion_reps: d as usize,
                    }
                }
            },
            (Some(motion), None, None) => {
                let cmd = NormalCmd::new(op).reps(reps).motion(motion);
                self.done(ctx, cmd)?;
            }
            (None, Some(scope), None) => self.state = State::TextObject { reps, op, scope },
            (None, None, Some(special)) => {
                let cmd = NormalCmd::new(op).reps(reps).special(special);
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
        op: Operator,
        motion_reps: usize,
    ) -> Result<()> {
        let (motion, scope, special) = (
            Motion::from(event),
            Scope::from(event),
            Secondary::from(event),
        );
        match (motion, scope, special) {
            (None, None, None) => match as_digit(&event) {
                None => self.reset(ctx)?,
                Some(d) => {
                    let new_motion_reps = motion_reps.saturating_mul(10).saturating_add(d as usize);
                    self.state = State::MotionReps {
                        reps,
                        op,
                        motion_reps: new_motion_reps,
                    };
                }
            },
            (Some(motion), None, None) => {
                let cmd = NormalCmd::new(op)
                    .reps(reps.saturating_mul(motion_reps))
                    .motion(motion);
                self.done(ctx, cmd)?;
            }
            (None, Some(scope), None) => {
                self.state = State::TextObject {
                    reps: reps.saturating_mul(motion_reps),
                    op,
                    scope,
                }
            }
            (None, None, Some(special)) => {
                let cmd = NormalCmd::new(op).special(special);
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
        op: Operator,
        scope: Scope,
    ) -> Result<()> {
        match Kind::from(event) {
            None => self.reset(ctx)?,
            Some(kind) => {
                let text_object = TextObject { scope, kind };
                let cmd = NormalCmd::new(op).reps(reps).text_object(text_object);
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

fn operator(event: KeyEvent) -> Option<Operator> {
    match event.code {
        KeyCode::Char('i') => Some(SysOp::EnterInsert(InsertPoint::Curr).into()),
        KeyCode::Char('I') => Some(SysOp::EnterInsert(InsertPoint::First).into()),
        KeyCode::Char('a') => Some(SysOp::EnterInsert(InsertPoint::Next).into()),
        KeyCode::Char('A') => Some(SysOp::EnterInsert(InsertPoint::Last).into()),
        KeyCode::Char(':') => Some(SysOp::EnterEx(ExMode::Colon).into()),
        KeyCode::Char('/') => Some(SysOp::EnterEx(ExMode::SearchForward).into()),
        KeyCode::Char('?') => Some(SysOp::EnterEx(ExMode::SearchBackward).into()),
        KeyCode::Char('Z') => Some(SysOp::BufferOp.into()),
        _ => Motion::from(event).map(Operator::Move),
    }
}
