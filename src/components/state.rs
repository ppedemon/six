use crate::components::SessionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStyle {
    None,
    Italic,
    Bold,
    ItalicBold,
}

pub struct Status {
    pub msg: String,
    pub level: Level,
    pub text_style: TextStyle,
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
            text_style: TextStyle::None,
            cmd: "".to_string(),
            ruler: "".to_string(),
        }
    }

    pub fn clear_msg(&mut self) {
        self.text_style = TextStyle::None;
        self.msg.clear();
    }

    pub fn clear_cmd(&mut self) {
        self.cmd.clear();
    }

    pub fn clear_ruler(&mut self) {
        self.ruler.clear();
    }

    pub fn set_msg(&mut self, level: Level, msg: &str) {
        self.set_styled_msg(level, TextStyle::None, msg);
    }

    pub fn set_styled_msg(&mut self, level: Level, text_style: TextStyle, msg: &str) {
        self.level = level;
        self.text_style = text_style;
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
    pub session_id: SessionId,
    pub char_at_cursor: Option<char>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            focus: Focus::Session,
            quit: false,
            session_id: 0,
            char_at_cursor: None,
        }
    }
}
