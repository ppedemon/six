mod buffer;
mod config;
mod display;
mod editor;
mod insert_log;
mod registers;
mod repeat;
mod search;
mod session;

use std::collections::HashMap;

pub use buffer::Buffer;
pub use config::Config;
pub use display::{DisplayBuffer, DisplayLine, DisplayLineRef};
pub use editor::{Editor, Focus, Level, Status, TextStyle};
pub use insert_log::InsertLog;
pub use registers::{Register, Registers};
pub use repeat::{RepeatBuffer, RepeatBufferItem};
pub use search::LastSearch;
pub use session::{BufferName, BufferView, Coords, ExSession, ExState, Mode, Session, Viewport};

pub type SessionId = usize;
pub type BufferId = usize;

pub struct EditorCtx {
    pub config: Config,
    pub editor: Editor,

    pub next_session_id: usize,
    pub next_buf_id: usize,

    pub sessions: HashMap<SessionId, (Session, BufferView)>,
    pub buffers: HashMap<BufferId, Buffer>,

    pub ex_session: ExSession,
    pub ex_buffer_view: BufferView,

    pub status: Status,
    pub registers: Registers,
    pub repbuf: RepeatBuffer,
    pub search: LastSearch,
}

impl EditorCtx {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            editor: Editor::new(),

            next_session_id: 0,
            next_buf_id: 0,

            sessions: HashMap::new(),
            buffers: HashMap::new(),

            ex_session: ExSession::new(),
            ex_buffer_view: BufferView::empty(),

            status: Status::new(),
            registers: Registers::empty(),
            repbuf: RepeatBuffer::new(),
            search: LastSearch::empty(),
        }
    }

    pub fn spawn_session(&mut self, session: Session, buf_view: BufferView) -> SessionId {
        let session_id = self.next_session_id;
        self.next_session_id += 1;
        self.sessions.insert(session_id, (session, buf_view));
        session_id
    }

    pub fn spawn_buffer(&mut self, buffer: Buffer) -> BufferId {
        let buf_id = self.next_buf_id;
        self.next_buf_id += 1;
        self.buffers.insert(buf_id, buffer);
        buf_id
    }
}

#[macro_export]
macro_rules! active_session {
    (mut $ctx:expr) => {{
        let (session, buf_view) = $ctx.sessions.get_mut(&$ctx.editor.session_id).unwrap();
        (session, buf_view)
    }};

    ($ctx:expr) => {{
        let (session, buf_view) = $ctx.sessions.get(&$ctx.editor.session_id).unwrap();
        (session, buf_view)
    }};
}

#[macro_export]
macro_rules! active_session_and_buffer {
    (mut $ctx:expr) => {{
        let (session, buf_view) = $ctx.sessions.get_mut(&$ctx.editor.session_id).unwrap();
        let buffer = $ctx.buffers.get_mut(&session.buf_id).unwrap();
        (session, buf_view, buffer)
    }};

    ($ctx:expr) => {{
        let (session, buf_view) = $ctx.sessions.get(&$ctx.editor.session_id).unwrap();
        let buffer = $ctx.buffers.get(&session.buf_id).unwrap();
        (session, buf_view, buffer)
    }};
}
