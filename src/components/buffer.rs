use std::path::PathBuf;
use ropey::Rope;

use crate::misc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferName {
    pub orig_name: String,
    pub file_path: PathBuf,
}

impl BufferName {
    pub fn new(orig_name: impl Into<String>, file_path: PathBuf) -> Self {
        Self {
            orig_name: orig_name.into(),
            file_path,
        }
    }
}

impl<T: Into<String>> From<T> for BufferName {
    fn from(orig_name: T) -> Self {
        let orig_name: String = orig_name.into();
        let file_path = misc::path::norm_filename(&orig_name);
        Self {
            orig_name,
            file_path,
        }
    }
}

pub struct Buffer {
    pub name: Option<BufferName>,
    pub dirty: bool,
    pub rope: Rope,
}

impl Buffer {
    pub fn new(name: BufferName, rope: Rope) -> Self {
        Self {
            name: Some(name),
            dirty: false,
            rope,
        }
    }

    pub fn empty() -> Self {
        Self {
            name: None,
            dirty: false,
            rope: Rope::new(),
        }
    }
}
