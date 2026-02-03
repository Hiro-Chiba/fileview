//! Unified error types for FileView
//!
//! Provides a consistent error handling approach across all modules.

use std::path::PathBuf;

/// Unified error type for FileView operations
#[derive(Debug, thiserror::Error)]
pub enum FileviewError {
    /// I/O errors (file operations, etc.)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Git-related errors
    #[error("Git error: {0}")]
    Git(String),

    /// MCP protocol errors
    #[error("MCP error: {0}")]
    Mcp(String),

    /// Preview rendering errors
    #[error("Preview error: {0}")]
    Preview(String),

    /// Path validation/security errors
    #[error("Path error: {path} - {reason}")]
    Path { path: PathBuf, reason: String },

    /// Configuration errors
    #[error("Config error: {0}")]
    Config(String),

    /// Plugin/Lua errors
    #[error("Plugin error: {0}")]
    Plugin(String),

    /// Token estimation errors
    #[error("Token error: {0}")]
    Token(String),

    /// Dependency analysis errors
    #[error("Dependency error: {0}")]
    Dependency(String),

    /// LSP/Analysis errors
    #[error("Analysis error: {0}")]
    Analysis(String),

    /// Project operation errors
    #[error("Project error: {0}")]
    Project(String),

    /// Generic internal errors
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience Result type using FileviewError
pub type Result<T> = std::result::Result<T, FileviewError>;

impl FileviewError {
    /// Create a Git error
    pub fn git(msg: impl Into<String>) -> Self {
        Self::Git(msg.into())
    }

    /// Create an MCP error
    pub fn mcp(msg: impl Into<String>) -> Self {
        Self::Mcp(msg.into())
    }

    /// Create a Preview error
    pub fn preview(msg: impl Into<String>) -> Self {
        Self::Preview(msg.into())
    }

    /// Create a Path error
    pub fn path(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::Path {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a Config error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a Plugin error
    pub fn plugin(msg: impl Into<String>) -> Self {
        Self::Plugin(msg.into())
    }

    /// Create a Token error
    pub fn token(msg: impl Into<String>) -> Self {
        Self::Token(msg.into())
    }

    /// Create a Dependency error
    pub fn dependency(msg: impl Into<String>) -> Self {
        Self::Dependency(msg.into())
    }

    /// Create an Analysis error
    pub fn analysis(msg: impl Into<String>) -> Self {
        Self::Analysis(msg.into())
    }

    /// Create a Project error
    pub fn project(msg: impl Into<String>) -> Self {
        Self::Project(msg.into())
    }

    /// Create an Internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

/// Extension trait for converting anyhow errors to FileviewError
pub trait AnyhowExt<T> {
    /// Convert to FileviewError with a context message
    fn map_fileview_err(self, f: impl FnOnce(anyhow::Error) -> FileviewError) -> Result<T>;
}

impl<T> AnyhowExt<T> for anyhow::Result<T> {
    fn map_fileview_err(self, f: impl FnOnce(anyhow::Error) -> FileviewError) -> Result<T> {
        self.map_err(f)
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
