use crossterm::event::{Event, KeyCode, KeyEvent};

use crate::{
    cmd::{Arg, Cmd, MotionMode, Operator, TextObject, TextObjectScope},
    components::EditorCtx,
    digraphs,
    systems::input::{
        evt::{self, Pretty},
        handler::dispatch_cmd,
        parsers::*,
        trie::FindResult,
    },
};

enum State {
    Init,
    Reg1,
    Reg2,
    RegReps,
    Reps,
    RepsReg1,
    Op,
    ArgInit,
    ArgReps,
    ArgRepsMode,
    ArgMode,
    ArgModeReps,
    ArgMotion,
    Digraph0 { parsing_op: bool },
    Digraph1 { parsing_op: bool, c: char },
    ToKind,
}

pub struct NormalInputHandler {
    state: State,
    reg: Option<char>,
    reps: Option<usize>,
    op: Operator,
    arg_reps: Option<usize>,
    mode: Option<MotionMode>,
    to_scope: Option<TextObjectScope>,
    input: Vec<KeyEvent>,
    cmd_buffer: String,
}

impl NormalInputHandler {
    pub fn new() -> Self {
        Self {
            state: State::Init,
            reg: None,
            reps: None,
            op: Operator::Nop,
            arg_reps: None,
            mode: None,
            to_scope: None,
            input: Vec::with_capacity(256),
            cmd_buffer: String::with_capacity(256),
        }
    }

    fn reset(&mut self, ctx: &mut EditorCtx) {
        ctx.status.clear_cmd();
        self.state = State::Init;
        self.reg = None;
        self.reps = None;
        self.op = Operator::Nop;
        self.arg_reps = None;
        self.mode = None;
        self.to_scope = None;
        self.input.clear();
        self.cmd_buffer.clear();
    }

    fn done(&mut self, ctx: &mut EditorCtx, cmd: Cmd) {
        self.reset(ctx);
        dispatch_cmd(ctx, cmd)
    }

    pub fn handle_event(&mut self, ctx: &mut EditorCtx, evt: Event) {
        match evt {
            Event::Key(key_evt) => self.handle_key(ctx, key_evt),
            _ => {}
        }
    }

    fn handle_key(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        if evt.code == KeyCode::Esc {
            self.reset(ctx);
        }

        self.cmd_buffer.push_str(&evt.pretty().to_string());
        ctx.status.set_cmd(&self.cmd_buffer);

        match self.state {
            State::Init => self.handle_init(ctx, evt),
            State::Reg1 => self.handle_reg1(ctx, evt),
            State::Reg2 => self.handle_reg2(ctx, evt),
            State::RegReps => self.handle_reg_reps(ctx, evt),
            State::Reps => self.handle_reps(ctx, evt),
            State::RepsReg1 => self.handle_reps_reg1(ctx, evt),
            State::Op => self.handle_op(ctx, evt),
            State::ArgInit => self.handle_arg_init(ctx, evt),
            State::ArgReps => self.handle_arg_reps(ctx, evt),
            State::ArgRepsMode => self.handle_arg_reps_mode(ctx, evt),
            State::ArgMode => self.handle_arg_mode(ctx, evt),
            State::ArgModeReps => self.handle_arg_mode_reps(ctx, evt),
            State::ArgMotion => self.handle_arg_motion(ctx, evt),
            State::Digraph0 { parsing_op } => self.handle_digraph0(ctx, evt, parsing_op),
            State::Digraph1 { parsing_op, c } => self.handle_digraph1(ctx, evt, parsing_op, c),
            State::ToKind => self.handle_to_kind(ctx, evt),
        }
    }

    fn handle_init(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        let (starts_reg, digit, op) = (
            starts_reg(evt),
            parse_non_zero_digit(evt),
            parse_op(self.reps, &[evt]),
        );

        match (starts_reg, digit, op) {
            (true, None, FindResult::Miss) => self.state = State::Reg1,
            (false, Some(d), _) => {
                self.reps = Some(d as usize);
                self.state = State::Reps;
            }
            (false, None, FindResult::Partial) => {
                self.input.push(evt);
                self.state = State::Op;
            }
            (false, None, FindResult::Hit(OpSpec { op, needs_arg })) => {
                if needs_arg {
                    self.op = op;
                    self.input.clear();
                    self.state = State::ArgInit;
                } else {
                    let cmd = Cmd::new(op);
                    self.done(ctx, cmd);
                }
            }
            _ => self.reset(ctx),
        }
    }

    fn handle_reg1(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        match parse_reg(evt) {
            Some(reg) => {
                self.reg = Some(reg);
                self.state = State::Reg2;
            }
            None => self.reset(ctx),
        }
    }

    fn handle_reg2(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        let (digit, op) = (parse_non_zero_digit(evt), parse_op(self.reps, &[evt]));

        match (digit, op) {
            (Some(d), _) => {
                self.reps = Some(d as usize);
                self.state = State::RegReps;
            }
            (None, FindResult::Partial) => {
                self.input.push(evt);
                self.state = State::Op;
            }
            (None, FindResult::Hit(OpSpec { op, needs_arg })) => {
                if needs_arg {
                    self.op = op;
                    self.input.clear();
                    self.state = State::ArgInit;
                } else {
                    let cmd = Cmd::new(op).reg(self.reg);
                    self.done(ctx, cmd);
                }
            }
            _ => self.reset(ctx),
        }
    }

    fn handle_reg_reps(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        let (digit, op) = (parse_non_zero_digit(evt), parse_op(self.reps, &[evt]));

        match (digit, op) {
            (Some(d), _) => {
                self.reps = self
                    .reps
                    .map(|reps| reps.saturating_mul(10).saturating_add(d as usize));
            }
            (None, FindResult::Partial) => {
                self.input.push(evt);
                self.state = State::Op;
            }
            (None, FindResult::Hit(OpSpec { op, needs_arg })) => {
                if needs_arg {
                    self.op = op;
                    self.input.clear();
                    self.state = State::ArgInit;
                } else {
                    let cmd = Cmd::new(op).reg(self.reg).reps(self.reps);
                    self.done(ctx, cmd);
                }
            }
            _ => self.reset(ctx),
        }
    }

    fn handle_reps(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        let (starts_reg, digit, op) = (
            starts_reg(evt),
            parse_digit(evt),
            parse_op(self.reps, &[evt]),
        );

        match (starts_reg, digit, op) {
            (true, None, FindResult::Miss) => self.state = State::RepsReg1,
            (false, Some(d), _) => {
                self.reps = self
                    .reps
                    .map(|reps| reps.saturating_mul(10).saturating_add(d as usize))
            }
            (false, None, FindResult::Partial) => {
                self.input.push(evt);
                self.state = State::Op;
            }
            (false, None, FindResult::Hit(OpSpec { op, needs_arg })) => {
                if needs_arg {
                    self.op = op;
                    self.input.clear();
                    self.state = State::ArgInit;
                } else {
                    let cmd = Cmd::new(op).reps(self.reps);
                    self.done(ctx, cmd);
                }
            }
            _ => self.reset(ctx),
        }
    }

    fn handle_reps_reg1(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        match parse_reg(evt) {
            Some(reg) => {
                self.reg = Some(reg);
                self.input.clear();
                self.state = State::Op;
            }
            None => self.reset(ctx),
        }
    }

    fn handle_op(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        if digraph_allowed(&self.input) && starts_digraph(evt) {
            self.state = State::Digraph0 { parsing_op: true };
            return;
        }

        self.input.push(evt);

        match parse_op(self.reps, &self.input) {
            FindResult::Miss => self.reset(ctx),
            FindResult::Partial => {}
            FindResult::Hit(OpSpec { op, needs_arg }) => {
                if needs_arg {
                    self.op = op;
                    self.input.clear();
                    self.state = State::ArgInit;
                } else {
                    let cmd = Cmd::new(op).reg(self.reg).reps(self.reps);
                    self.done(ctx, cmd);
                }
            }
        }
    }

    fn handle_arg_init(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        let (digit, mode, motion, to_scope) = (
            parse_non_zero_digit(evt),
            parse_motion_mode(evt),
            parse_motion_arg(self.op, self.arg_reps, &[evt]),
            parse_textobject_scope(evt),
        );

        match (digit, mode, motion, to_scope) {
            (Some(d), None, _, None) => {
                self.arg_reps = Some(d as usize);
                self.state = State::ArgReps;
            }
            (None, Some(mode), FindResult::Miss, None) => {
                self.mode = Some(mode);
                self.state = State::ArgMode;
            }
            (None, None, FindResult::Partial, None) => {
                self.input.push(evt);
                self.state = State::ArgMotion;
            }
            (None, None, FindResult::Hit(motion), None) => {
                let arg = Arg::motion(self.arg_reps, self.mode, motion);
                let cmd = Cmd::new(self.op).reg(self.reg).reps(self.reps).arg(arg);
                self.done(ctx, cmd);
            }
            (None, None, FindResult::Miss, Some(scope)) => {
                self.to_scope = Some(scope);
                self.state = State::ToKind;
            }
            _ => self.reset(ctx),
        }
    }

    fn handle_arg_reps(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        let (digit, mode, motion, to_scope) = (
            parse_digit(evt),
            parse_motion_mode(evt),
            parse_motion_arg(self.op, self.arg_reps, &[evt]),
            parse_textobject_scope(evt),
        );

        match (digit, mode, motion, to_scope) {
            (Some(d), None, _, None) => {
                self.arg_reps = self
                    .arg_reps
                    .map(|reps| reps.saturating_mul(10).saturating_add(d as usize));
            }
            (None, Some(mode), FindResult::Miss, None) => {
                self.mode = Some(mode);
                self.state = State::ArgRepsMode;
            }
            (None, None, FindResult::Partial, None) => {
                self.input.push(evt);
                self.state = State::ArgMotion;
            }
            (None, None, FindResult::Hit(motion), None) => {
                let arg = Arg::motion(self.arg_reps, self.mode, motion);
                let cmd = Cmd::new(self.op).reg(self.reg).reps(self.reps).arg(arg);
                self.done(ctx, cmd);
            }
            (None, None, FindResult::Miss, Some(scope)) => {
                self.to_scope = Some(scope);
                self.state = State::ToKind;
            }
            _ => self.reset(ctx),
        }
    }

    fn handle_arg_reps_mode(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        let (motion, to_scope) = (
            parse_motion_arg(self.op, self.arg_reps, &[evt]),
            parse_textobject_scope(evt),
        );

        match (motion, to_scope) {
            (FindResult::Partial, None) => {
                self.input.push(evt);
                self.state = State::ArgMotion;
            }
            (FindResult::Hit(motion), None) => {
                let arg = Arg::motion(self.arg_reps, self.mode, motion);
                let cmd = Cmd::new(self.op).reg(self.reg).reps(self.reps).arg(arg);
                self.done(ctx, cmd);
            }
            (FindResult::Miss, Some(scope)) => {
                self.to_scope = Some(scope);
                self.state = State::ToKind;
            }
            _ => self.reset(ctx),
        }
    }

    fn handle_arg_mode(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        let (digit, motion, to_scope) = (
            parse_non_zero_digit(evt),
            parse_motion_arg(self.op, self.arg_reps, &[evt]),
            parse_textobject_scope(evt),
        );

        match (digit, motion, to_scope) {
            (Some(d), _, None) => {
                self.arg_reps = Some(d as usize);
                self.state = State::ArgModeReps;
            }
            (None, FindResult::Partial, None) => {
                self.input.push(evt);
                self.state = State::ArgMotion;
            }
            (None, FindResult::Hit(motion), None) => {
                let arg = Arg::motion(self.arg_reps, self.mode, motion);
                let cmd = Cmd::new(self.op).reg(self.reg).reps(self.reps).arg(arg);
                self.done(ctx, cmd);
            }
            (None, FindResult::Miss, Some(scope)) => {
                self.to_scope = Some(scope);
                self.state = State::ToKind;
            }
            _ => self.reset(ctx),
        }
    }

    fn handle_arg_mode_reps(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        let (digit, motion, to_scope) = (
            parse_digit(evt),
            parse_motion_arg(self.op, self.arg_reps, &[evt]),
            parse_textobject_scope(evt),
        );

        match (digit, motion, to_scope) {
            (Some(d), _, None) => {
                self.arg_reps = self
                    .arg_reps
                    .map(|reps| reps.saturating_mul(10).saturating_add(d as usize));
            }
            (None, FindResult::Partial, None) => {
                self.input.push(evt);
                self.state = State::ArgMotion;
            }
            (None, FindResult::Hit(motion), None) => {
                let arg = Arg::motion(self.arg_reps, self.mode, motion);
                let cmd = Cmd::new(self.op).reg(self.reg).reps(self.reps).arg(arg);
                self.done(ctx, cmd);
            }
            (None, FindResult::Miss, Some(scope)) => {
                self.to_scope = Some(scope);
                self.state = State::ToKind;
            }
            _ => self.reset(ctx),
        }
    }

    fn handle_arg_motion(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        if digraph_allowed(&self.input) && starts_digraph(evt) {
            self.state = State::Digraph0 { parsing_op: false };
            return;
        }

        self.input.push(evt);

        match parse_motion_arg(self.op, self.arg_reps, &self.input) {
            FindResult::Partial => {}
            FindResult::Hit(motion) => {
                let arg = Arg::motion(self.arg_reps, self.mode, motion);
                let cmd = Cmd::new(self.op).reg(self.reg).reps(self.reps).arg(arg);
                self.done(ctx, cmd);
            }
            FindResult::Miss => self.reset(ctx),
        }
    }

    fn handle_to_kind(&mut self, ctx: &mut EditorCtx, evt: KeyEvent) {
        match parse_textobject_kind(evt) {
            None => self.reset(ctx),
            Some(kind) => {
                if let Some(scope) = self.to_scope {
                    let text_object = TextObject::new(scope, kind);
                    let arg = Arg::text_object(self.arg_reps, self.mode, text_object);
                    let cmd = Cmd::new(self.op).reg(self.reg).reps(self.reps).arg(arg);
                    self.done(ctx, cmd);
                } else {
                    self.reset(ctx);
                }
            }
        }
    }

    fn handle_digraph0(&mut self, ctx: &mut EditorCtx, evt: KeyEvent, parsing_op: bool) {
        match evt.code {
            KeyCode::Char(c) => self.state = State::Digraph1 { parsing_op, c },
            _ => self.reset(ctx),
        }
    }

    fn handle_digraph1(&mut self, ctx: &mut EditorCtx, evt: KeyEvent, parsing_op: bool, c0: char) {
        let dg = evt.code.as_char().and_then(|c1| digraphs::get(c0, c1));

        if let Some(c) = dg {
            let fake_evt = evt::char(c);
            self.input.push(fake_evt);

            if parsing_op {
                match parse_op(self.reps, &self.input) {
                    FindResult::Miss => self.reset(ctx),
                    FindResult::Partial => self.state = State::Op,
                    FindResult::Hit(OpSpec { op, needs_arg }) => {
                        if needs_arg {
                            self.op = op;
                            self.input.clear();
                            self.state = State::ArgInit;
                        } else {
                            let cmd = Cmd::new(op).reg(self.reg).reps(self.reps);
                            self.done(ctx, cmd);
                        }
                    }
                }
            } else {
                match parse_motion_arg(self.op, self.arg_reps, &self.input) {
                    FindResult::Partial => self.state = State::ArgMotion,
                    FindResult::Hit(motion) => {
                        let arg = Arg::motion(self.arg_reps, self.mode, motion);
                        let cmd = Cmd::new(self.op).reg(self.reg).reps(self.reps).arg(arg);
                        self.done(ctx, cmd);
                    }
                    FindResult::Miss => self.reset(ctx),
                }
            }
        } else {
            self.reset(ctx);
        }
    }
}
