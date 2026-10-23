//! Callback execution (--on-select option)

use std::path::Path;
use std::process::Command;

pub mod placeholder {
    /// Full path: /path/to/file.txt
    pub const PATH: &str = "{path}";
    /// Directory: /path/to
    pub const DIR: &str = "{dir}";
    /// Filename with extension: file.txt
    pub const NAME: &str = "{name}";
    /// Filename without extension: file
    pub const STEM: &str = "{stem}";
    /// Extension only: txt
    pub const EXT: &str = "{ext}";
}

#[derive(Debug, Clone)]
pub struct Callback {
    /// Command template with placeholders
    command: String,
    /// Whether to run in background (don't wait for completion)
    background: bool,
    /// Shell to use (default: sh -c)
    shell: Option<String>,
}

impl Callback {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            background: false,
            shell: None,
        }
    }

    pub fn background(mut self, bg: bool) -> Self {
        self.background = bg;
        self
    }

    pub fn shell(mut self, shell: impl Into<String>) -> Self {
        self.shell = Some(shell.into());
        self
    }

    pub fn expand(&self, path: &Path) -> String {
        let path_str = path.display().to_string();
        let dir_str = path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let name_str = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let stem_str = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext_str = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        self.command
            .replace(placeholder::PATH, &shell_escape(&path_str))
            .replace(placeholder::DIR, &shell_escape(&dir_str))
            .replace(placeholder::NAME, &shell_escape(&name_str))
            .replace(placeholder::STEM, &shell_escape(&stem_str))
            .replace(placeholder::EXT, &shell_escape(&ext_str))
    }

    pub fn execute(&self, path: &Path) -> anyhow::Result<CallbackResult> {
        let expanded = self.expand(path);

        let shell = self.shell.as_deref().unwrap_or("sh");
        let mut cmd = Command::new(shell);
        cmd.arg("-c").arg(&expanded);

        if self.background {
            let child = cmd.spawn()?;
            Ok(CallbackResult::Spawned {
                pid: child.id(),
                command: expanded,
            })
        } else {
            let output = cmd.output()?;
            Ok(CallbackResult::Completed {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }

    pub fn execute_many(&self, paths: &[&Path]) -> Vec<anyhow::Result<CallbackResult>> {
        paths.iter().map(|p| self.execute(p)).collect()
    }
}

#[derive(Debug)]
pub enum CallbackResult {
    Spawned {
        pid: u32,
        command: String,
    },
    Completed {
        success: bool,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    },
}

impl CallbackResult {
    pub fn is_success(&self) -> bool {
        match self {
            Self::Spawned { .. } => true,
            Self::Completed { success, .. } => *success,
        }
    }
}

fn shell_escape(s: &str) -> String {
    // Simple escaping: wrap in single quotes, escape existing single quotes
    if s.contains('\'') {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        format!("'{}'", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_placeholder_expansion() {
        let callback = Callback::new("echo {path} {name} {stem} {ext}");
        let path = PathBuf::from("/home/user/document.txt");
        let expanded = callback.expand(&path);

        assert!(expanded.contains("'/home/user/document.txt'"));
        assert!(expanded.contains("'document.txt'"));
        assert!(expanded.contains("'document'"));
        assert!(expanded.contains("'txt'"));
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("simple"), "'simple'");
        assert_eq!(shell_escape("with space"), "'with space'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_callback_builder() {
        let callback = Callback::new("test").background(true).shell("bash");

        assert!(callback.background);
        assert_eq!(callback.shell, Some("bash".to_string()));
    }
}
