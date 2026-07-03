use hecs::Entity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

pub struct Status {
    pub msg: String,
    pub level: Level,
    pub cmd: String,
    pub ruler: String,
}

impl Status {
    pub const MAX_CMD: usize = 10;
    pub const MAX_RULER: usize = 20;

    pub fn new() -> Self {
        Self {
            msg: "".to_string(),
            level: Level::Info,
            cmd: "".to_string(),
            ruler: "".to_string(),
        }
    }

    pub fn clear_msg(&mut self) {
        self.msg.clear();
    }

    pub fn clear_cmd(&mut self) {
        self.cmd.clear();
    }

    pub fn clear_ruler(&mut self) {
        self.ruler.clear();
    }

    pub fn set_msg(&mut self, level: Level, msg: &str) {
        self.level = level;
        self.msg.clear();
        self.msg.push_str(msg);
    }

    pub fn set_cmd(&mut self, cmd_msg: &str) {
        self.cmd.clear();
        self.cmd.push_str(cmd_msg);
    }

    pub fn set_ruler(&mut self, ruler_msg: &str) {
        self.ruler.clear();
        self.ruler.push_str(ruler_msg);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Ex,
    Session,
}

pub struct EditorState {
    pub focus: Focus,
    pub quit: bool,
    pub session_id: Entity,
    pub char_at_cursor: Option<char>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            focus: Focus::Session,
            quit: false,
            session_id: Entity::DANGLING,
            char_at_cursor: None,
        }
    }
}
