use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExError {
    #[error("Not an editor command: {cmd}")]
    ParseError { cmd: String },

    #[error("No write since last change (add ! to override)")]
    UnsavedChanges,

    #[error("No file name")]
    NoFileName,

    #[error("File exists (add ! to override)")]
    FileExists,

    #[error("Use ! to write partial buffer")]
    PartialWrite,

    #[error("Pattern not found")]
    PatternNotFound,

    #[error("Invalid range")]
    InvalidRange,

    #[error("Unsupported Address: {address}")]
    UnsupportedAddress { address: String },

    #[error("Command does not take arguments")]
    UnsupportedArgs,

    #[error("Unsupported shell command: {cmd}")]
    UnsuportedShellCommand { cmd: String },

    #[error("Mark not found: {mark}")]
    MarkNotFound { mark: char },

    #[error("{msg}")]
    InvalidArgs { msg: String },

    #[error("I/O error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },
}

impl ExError {
    pub fn invalid_args(msg: &str) -> Self {
        Self::InvalidArgs {
            msg: msg.to_string(),
        }
    }
}
