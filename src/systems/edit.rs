mod batch;
mod buffer;
mod session;

pub use buffer::insert_char;
pub use session::{handle_edit, post_edit, clear_ex};
