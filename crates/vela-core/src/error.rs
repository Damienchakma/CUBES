use std::path::PathBuf;

/// Errors produced by the project and resource core.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid project manifest: {0}")]
    InvalidManifest(String),
    #[error("project already exists: {0}")]
    ProjectExists(PathBuf),
    #[error("path escapes the project sandbox: {0}")]
    SandboxViolation(PathBuf),
    #[error("unsupported file type: {0}")]
    UnsupportedFileType(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
