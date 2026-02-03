//! Event hooks system
//!
//! Executes user-defined scripts in response to file operations and events.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

/// Hook event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    /// Triggered after a file is created
    OnCreate,
    /// Triggered after a file is deleted
    OnDelete,
    /// Triggered after a file is renamed
    OnRename,
    /// Triggered when changing directory
    OnCd,
    /// Triggered when the application starts
    OnStart,
    /// Triggered when the application exits
    OnExit,
}

impl HookEvent {
    /// Get the config key for this hook event
    pub fn config_key(&self) -> &'static str {
        match self {
            HookEvent::OnCreate => "on_create",
            HookEvent::OnDelete => "on_delete",
            HookEvent::OnRename => "on_rename",
            HookEvent::OnCd => "on_cd",
            HookEvent::OnStart => "on_start",
            HookEvent::OnExit => "on_exit",
        }
    }
}

/// Hook configuration from config file
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HooksConfig {
    /// Script to run after file creation
    pub on_create: Option<String>,
    /// Script to run after file deletion
    pub on_delete: Option<String>,
    /// Script to run after file rename
    pub on_rename: Option<String>,
    /// Script to run on directory change
    pub on_cd: Option<String>,
    /// Script to run on application start
    pub on_start: Option<String>,
    /// Script to run on application exit
    pub on_exit: Option<String>,
}

impl HooksConfig {
    /// Get the script path for a hook event
    pub fn get(&self, event: HookEvent) -> Option<&str> {
        match event {
            HookEvent::OnCreate => self.on_create.as_deref(),
            HookEvent::OnDelete => self.on_delete.as_deref(),
            HookEvent::OnRename => self.on_rename.as_deref(),
            HookEvent::OnCd => self.on_cd.as_deref(),
            HookEvent::OnStart => self.on_start.as_deref(),
            HookEvent::OnExit => self.on_exit.as_deref(),
        }
    }
}

/// Context for hook execution
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    /// Target file path
    pub path: Option<PathBuf>,
    /// Old path (for rename operations)
    pub old_path: Option<PathBuf>,
    /// Current directory
    pub dir: Option<PathBuf>,
    /// Selected files (for multi-select operations)
    pub selected: Vec<PathBuf>,
}

/// Hook executor
pub struct HookExecutor {
    config: HooksConfig,
}

impl HookExecutor {
    /// Create a new hook executor with the given configuration
    pub fn new(config: HooksConfig) -> Self {
        Self { config }
    }

    /// Execute a hook if configured
    ///
    /// This runs asynchronously (non-blocking) so it doesn't slow down the UI.
    pub fn execute(&self, event: HookEvent, context: &HookContext) {
        if let Some(script) = self.config.get(event) {
            let expanded = Self::expand_script(script, context);
            Self::run_script_async(&expanded, context);
        }
    }

    /// Execute a hook synchronously (blocking)
    ///
    /// Use this only when you need to wait for the hook to complete.
    pub fn execute_sync(&self, event: HookEvent, context: &HookContext) -> anyhow::Result<()> {
        if let Some(script) = self.config.get(event) {
            let expanded = Self::expand_script(script, context);
            Self::run_script_sync(&expanded, context)?;
        }
        Ok(())
    }

    /// Expand path in script (~ to home directory)
    fn expand_script(script: &str, _context: &HookContext) -> String {
        if script.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                return script.replacen('~', &home.display().to_string(), 1);
            }
        }
        script.to_string()
    }

    /// Build environment variables for hook execution
    fn build_env(context: &HookContext) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // FILEVIEW_PATH: target file path
        if let Some(ref path) = context.path {
            env.insert("FILEVIEW_PATH".to_string(), path.display().to_string());
        }

        // FILEVIEW_OLD_PATH: old path (for rename)
        if let Some(ref old_path) = context.old_path {
            env.insert(
                "FILEVIEW_OLD_PATH".to_string(),
                old_path.display().to_string(),
            );
        }

        // FILEVIEW_DIR: current directory
        if let Some(ref dir) = context.dir {
            env.insert("FILEVIEW_DIR".to_string(), dir.display().to_string());
        }

        // FILEVIEW_SELECTED: newline-separated list of selected files
        if !context.selected.is_empty() {
            let selected: Vec<String> = context
                .selected
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            env.insert("FILEVIEW_SELECTED".to_string(), selected.join("\n"));
        }

        env
    }

    /// Run a script asynchronously
    fn run_script_async(script: &str, context: &HookContext) {
        let script = script.to_string();
        let env = Self::build_env(context);

        std::thread::spawn(move || {
            let _ = Self::execute_script(&script, &env);
        });
    }

    /// Run a script synchronously
    fn run_script_sync(script: &str, context: &HookContext) -> anyhow::Result<()> {
        let env = Self::build_env(context);
        Self::execute_script(script, &env)
    }

    /// Execute a script with environment variables
    fn execute_script(script: &str, env: &HashMap<String, String>) -> anyhow::Result<()> {
        let path = Path::new(script);

        // Check if script exists and is executable
        if !path.exists() {
            return Ok(()); // Silently skip non-existent scripts
        }

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", script]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", script]);
            c
        };

        // Set environment variables
        for (key, value) in env {
            cmd.env(key, value);
        }

        // Run and ignore output
        let _ = cmd
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        Ok(())
    }

    /// Create a context for a file operation
    pub fn context_for_file(path: &Path, dir: &Path) -> HookContext {
        HookContext {
            path: Some(path.to_path_buf()),
            old_path: None,
            dir: Some(dir.to_path_buf()),
            selected: Vec::new(),
        }
    }

    /// Create a context for a rename operation
    pub fn context_for_rename(old_path: &Path, new_path: &Path, dir: &Path) -> HookContext {
        HookContext {
            path: Some(new_path.to_path_buf()),
            old_path: Some(old_path.to_path_buf()),
            dir: Some(dir.to_path_buf()),
            selected: Vec::new(),
        }
    }

    /// Create a context for a directory change
    pub fn context_for_cd(dir: &Path) -> HookContext {
        HookContext {
            path: None,
            old_path: None,
            dir: Some(dir.to_path_buf()),
            selected: Vec::new(),
        }
    }

    /// Create a context for multi-file operations
    pub fn context_for_selected(selected: &[PathBuf], dir: &Path) -> HookContext {
        HookContext {
            path: selected.first().cloned(),
            old_path: None,
            dir: Some(dir.to_path_buf()),
            selected: selected.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_config_default() {
        let config = HooksConfig::default();
        assert!(config.on_create.is_none());
        assert!(config.on_delete.is_none());
        assert!(config.on_rename.is_none());
        assert!(config.on_cd.is_none());
        assert!(config.on_start.is_none());
        assert!(config.on_exit.is_none());
    }

    #[test]
    fn test_hooks_config_get() {
        let config = HooksConfig {
            on_create: Some("/path/to/create.sh".to_string()),
            on_delete: Some("/path/to/delete.sh".to_string()),
            ..Default::default()
        };

        assert_eq!(config.get(HookEvent::OnCreate), Some("/path/to/create.sh"));
        assert_eq!(config.get(HookEvent::OnDelete), Some("/path/to/delete.sh"));
        assert_eq!(config.get(HookEvent::OnRename), None);
    }

    #[test]
    fn test_hook_context_for_file() {
        let path = PathBuf::from("/test/file.txt");
        let dir = PathBuf::from("/test");
        let context = HookExecutor::context_for_file(&path, &dir);

        assert_eq!(context.path, Some(path));
        assert_eq!(context.dir, Some(dir));
        assert!(context.old_path.is_none());
        assert!(context.selected.is_empty());
    }

    #[test]
    fn test_hook_context_for_rename() {
        let old_path = PathBuf::from("/test/old.txt");
        let new_path = PathBuf::from("/test/new.txt");
        let dir = PathBuf::from("/test");
        let context = HookExecutor::context_for_rename(&old_path, &new_path, &dir);

        assert_eq!(context.path, Some(new_path));
        assert_eq!(context.old_path, Some(old_path));
        assert_eq!(context.dir, Some(dir));
    }

    #[test]
    fn test_build_env() {
        let context = HookContext {
            path: Some(PathBuf::from("/test/file.txt")),
            old_path: Some(PathBuf::from("/test/old.txt")),
            dir: Some(PathBuf::from("/test")),
            selected: vec![PathBuf::from("/test/a.txt"), PathBuf::from("/test/b.txt")],
        };

        let env = HookExecutor::build_env(&context);

        assert_eq!(
            env.get("FILEVIEW_PATH"),
            Some(&"/test/file.txt".to_string())
        );
        assert_eq!(
            env.get("FILEVIEW_OLD_PATH"),
            Some(&"/test/old.txt".to_string())
        );
        assert_eq!(env.get("FILEVIEW_DIR"), Some(&"/test".to_string()));
        assert_eq!(
            env.get("FILEVIEW_SELECTED"),
            Some(&"/test/a.txt\n/test/b.txt".to_string())
        );
    }

    #[test]
    fn test_hook_event_config_key() {
        assert_eq!(HookEvent::OnCreate.config_key(), "on_create");
        assert_eq!(HookEvent::OnDelete.config_key(), "on_delete");
        assert_eq!(HookEvent::OnRename.config_key(), "on_rename");
        assert_eq!(HookEvent::OnCd.config_key(), "on_cd");
        assert_eq!(HookEvent::OnStart.config_key(), "on_start");
        assert_eq!(HookEvent::OnExit.config_key(), "on_exit");
    }

    #[test]
    fn test_expand_script_with_tilde() {
        let script = "~/.config/fileview/hooks/test.sh";
        let context = HookContext::default();
        let expanded = HookExecutor::expand_script(script, &context);

        // Should not start with ~ anymore (unless home dir lookup failed)
        if dirs::home_dir().is_some() {
            assert!(!expanded.starts_with('~'));
        }
    }
}
