use hecs::World;

mod buffer;
mod config;
mod display;
mod insert_log;
mod session;
mod state;

pub use buffer::{Buffer, BufferName};
pub use config::Config;
pub use display::{DisplayBuffer, DisplayLine, DisplayLineRef};
pub use insert_log::InsertLog;
pub use session::{BufferView, Coords, ExState, ExSession, Mode, Session, Viewport};
pub use state::{EditorState, Focus, Level, Status};

pub struct EditorCtx<'a> {
    pub world: &'a mut World,
    pub config_id: hecs::Entity,
    pub editor_id: hecs::Entity,
    pub ex_session_id: hecs::Entity,
    pub status_id: hecs::Entity,
}
