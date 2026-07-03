mod batch;
mod buffer;
mod session;

pub use batch::apply_insert_log;
pub use buffer::insert_char;
pub use session::{handle_edit, clear_ex};
