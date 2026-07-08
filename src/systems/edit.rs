mod batch;
mod buffer;
mod session;

pub use buffer::insert_char;
pub use session::{clear_ex, handle_edit, post_edit};
