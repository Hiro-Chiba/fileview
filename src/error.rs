//! Unified error types for FileView

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum FileviewError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("Preview error: {0}")]
    Preview(String),

    #[error("Path error: {path} - {reason}")]
    Path { path: PathBuf, reason: String },

    #[error("Config error: {0}")]
    Config(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Token error: {0}")]
    Token(String),

    #[error("Dependency error: {0}")]
    Dependency(String),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("Project error: {0}")]
    Project(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, FileviewError>;

impl FileviewError {
    pub fn git(msg: impl Into<String>) -> Self {
        Self::Git(msg.into())
    }

    pub fn mcp(msg: impl Into<String>) -> Self {
        Self::Mcp(msg.into())
    }

    pub fn preview(msg: impl Into<String>) -> Self {
        Self::Preview(msg.into())
    }

    pub fn path(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::Path {
            path: path.into(),
            reason: reason.into(),
        }
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn plugin(msg: impl Into<String>) -> Self {
        Self::Plugin(msg.into())
    }

    pub fn token(msg: impl Into<String>) -> Self {
        Self::Token(msg.into())
    }

    pub fn dependency(msg: impl Into<String>) -> Self {
        Self::Dependency(msg.into())
    }

    pub fn analysis(msg: impl Into<String>) -> Self {
        Self::Analysis(msg.into())
    }

    pub fn project(msg: impl Into<String>) -> Self {
        Self::Project(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = FileviewError::git("repository not found");
        assert_eq!(format!("{}", err), "Git error: repository not found");

        let err = FileviewError::path("/foo/bar", "outside root directory");
        assert_eq!(
            format!("{}", err),
            "Path error: /foo/bar - outside root directory"
        );
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: FileviewError = io_err.into();
        assert!(matches!(err, FileviewError::Io(_)));
    }

    #[test]
    fn test_result_type() {
        fn returns_result() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(returns_result().unwrap(), 42);
    }
}
