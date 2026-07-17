mod buffer;
mod config;
mod display;
mod insert_log;
mod registers;
mod repeat;
mod search;
mod session;
mod state;

use std::collections::HashMap;

pub use buffer::Buffer;
pub use config::Config;
pub use display::{DisplayBuffer, DisplayLine, DisplayLineRef};
pub use insert_log::InsertLog;
pub use registers::Registers;
pub use repeat::{RepeatBuffer, RepeatBufferItem};
pub use search::LastSearch;
pub use session::{BufferName, BufferView, Coords, ExSession, ExState, Mode, Session, Viewport};
pub use state::{EditorState, Focus, Level, Status};

pub type SessionId = usize;
pub type BufferId = usize;

pub struct EditorCtx {
    pub config: Config,
    pub editor: EditorState,

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
            editor: EditorState::new(),

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

    pub fn active_session(&self) -> (&Session, &BufferView) {
        let tuple = self.sessions.get(&self.editor.session_id).unwrap();
        (&tuple.0, &tuple.1)
    }

    pub fn active_session_mut(&mut self) -> (&mut Session, &mut BufferView) {
        let tuple = self.sessions.get_mut(&self.editor.session_id).unwrap();
        (&mut tuple.0, &mut tuple.1)
    }
}
