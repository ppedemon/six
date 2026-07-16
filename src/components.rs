use hecs::World;

mod buffer;
mod config;
mod display;
mod insert_log;
mod registers;
mod repeat;
mod search;
mod session;
mod state;

pub use buffer::Buffer;
pub use config::Config;
pub use display::{DisplayBuffer, DisplayLine, DisplayLineRef};
pub use insert_log::InsertLog;
pub use registers::Registers;
pub use repeat::{RepeatBuffer, RepeatBufferItem};
pub use search::LastSearch;
pub use session::{BufferName, BufferView, Coords, ExSession, ExState, Mode, Session, Viewport};
pub use state::{EditorState, Focus, Level, Status};

pub struct EditorCtx<'a> {
    pub world: &'a mut World,
    pub config_id: hecs::Entity,
    pub editor_id: hecs::Entity,
    pub ex_session_id: hecs::Entity,
    pub status_id: hecs::Entity,
    pub registers_id: hecs::Entity,
    pub repbuf_id: hecs::Entity,
    pub search_id: hecs::Entity,
}
