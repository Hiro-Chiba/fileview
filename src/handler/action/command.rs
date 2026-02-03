//! Custom command execution
//!
//! Executes user-defined shell commands with placeholder expansion.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::app::CommandsConfig;
use crate::core::AppState;

/// Result of command execution
#[derive(Debug)]
pub enum CommandResult {
    /// Command executed successfully
    Success(String),
    /// Command failed
    Error(String),
    /// Command not found in config
    NotFound,
}

/// Execute a custom command by name
///
/// # Arguments
/// * `name` - The command name as defined in config
/// * `config` - The commands configuration
/// * `file_path` - The current file path for placeholder expansion
/// * `selected_paths` - Selected file paths (for $S placeholder)
pub fn execute_command(
    name: &str,
    config: &CommandsConfig,
    file_path: Option<&Path>,
    selected_paths: &[std::path::PathBuf],
) -> CommandResult {
    let template = match config.get(name) {
        Some(t) => t,
        None => return CommandResult::NotFound,
    };

    // Expand placeholders
    let cmd = if let Some(path) = file_path {
        let mut expanded = CommandsConfig::expand(template, path);

        // Handle $S (selected files) - join paths with spaces, quoted
        if expanded.contains("$S") {
            let selected: Vec<String> = selected_paths
                .iter()
                .map(|p| format!("'{}'", p.display()))
                .collect();
            expanded = expanded.replace("$S", &selected.join(" "));
        }

        expanded
    } else {
        template.clone()
    };

    // Execute command via shell
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", &cmd]).output()
    } else {
        Command::new("sh").args(["-c", &cmd]).output()
    };

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                CommandResult::Success(stdout)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                CommandResult::Error(if stderr.is_empty() {
                    format!("Command failed with exit code: {:?}", output.status.code())
                } else {
                    stderr
                })
            }
        }
        Err(e) => CommandResult::Error(format!("Failed to execute command: {}", e)),
    }
}

/// Execute a command and wait for it to complete (for TUI restoration)
///
/// This spawns the command in a way that allows it to take over the terminal,
/// useful for commands like editors or pagers.
pub fn execute_interactive(
    name: &str,
    config: &CommandsConfig,
    file_path: Option<&Path>,
) -> CommandResult {
    let template = match config.get(name) {
        Some(t) => t,
        None => return CommandResult::NotFound,
    };

    let cmd = if let Some(path) = file_path {
        CommandsConfig::expand(template, path)
    } else {
        template.clone()
    };

    // For interactive commands, we need to spawn and wait
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", &cmd]).status()
    } else {
        Command::new("sh").args(["-c", &cmd]).status()
    };

    match status {
        Ok(status) => {
            if status.success() {
                CommandResult::Success(String::new())
            } else {
                CommandResult::Error(format!("Command exited with: {:?}", status.code()))
            }
        }
        Err(e) => CommandResult::Error(format!("Failed to execute command: {}", e)),
    }
}

/// Open a subshell in the current directory
///
/// This spawns the user's default shell in the current directory.
/// The fileview UI will be suspended until the subshell exits.
pub fn open_subshell(state: &mut AppState, focused_path: Option<&PathBuf>) {
    // Determine the target directory
    let dir = focused_path
        .and_then(|p| {
            if p.is_dir() {
                Some(p.clone())
            } else {
                p.parent().map(|pp| pp.to_path_buf())
            }
        })
        .unwrap_or_else(|| state.root.clone());

    // Get the user's shell
    let shell = std::env::var("SHELL").unwrap_or_else(|_| {
        if cfg!(target_os = "windows") {
            "cmd".to_string()
        } else {
            "/bin/sh".to_string()
        }
    });

    // Message for the user
    state.set_message(format!("Opening subshell in {}...", dir.display()));

    // Note: Actually spawning an interactive shell requires terminal handling
    // that goes beyond this simple implementation. The real implementation would
    // need to:
    // 1. Suspend the terminal UI
    // 2. Spawn the shell interactively
    // 3. Wait for the shell to exit
    // 4. Restore the terminal UI
    //
    // For now, we just show a message about how to use this feature
    // A full implementation would be handled in the event loop.
    state.set_message(format!(
        "Shell: {} (press Enter to spawn in {})",
        shell,
        dir.display()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_config(commands: Vec<(&str, &str)>) -> CommandsConfig {
        let mut map = HashMap::new();
        for (name, cmd) in commands {
            map.insert(name.to_string(), cmd.to_string());
        }
        CommandsConfig { commands: map }
    }

    #[test]
    fn test_command_not_found() {
        let config = create_config(vec![]);
        let result = execute_command("nonexistent", &config, None, &[]);
        assert!(matches!(result, CommandResult::NotFound));
    }

    #[test]
    fn test_simple_command() {
        let config = create_config(vec![("echo_test", "echo hello")]);
        let result = execute_command("echo_test", &config, None, &[]);
        match result {
            CommandResult::Success(output) => {
                assert!(output.contains("hello"));
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_placeholder_expansion() {
        let config = create_config(vec![("show_path", "echo $f")]);
        let path = PathBuf::from("/tmp/test.txt");
        let result = execute_command("show_path", &config, Some(&path), &[]);
        match result {
            CommandResult::Success(output) => {
                assert!(output.contains("/tmp/test.txt"));
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_directory_placeholder() {
        let config = create_config(vec![("show_dir", "echo $d")]);
        let path = PathBuf::from("/tmp/test.txt");
        let result = execute_command("show_dir", &config, Some(&path), &[]);
        match result {
            CommandResult::Success(output) => {
                assert!(output.contains("/tmp"));
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_filename_placeholder() {
        let config = create_config(vec![("show_name", "echo $n")]);
        let path = PathBuf::from("/tmp/test.txt");
        let result = execute_command("show_name", &config, Some(&path), &[]);
        match result {
            CommandResult::Success(output) => {
                assert!(output.contains("test.txt"));
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_stem_placeholder() {
        let config = create_config(vec![("show_stem", "echo $s")]);
        let path = PathBuf::from("/tmp/test.txt");
        let result = execute_command("show_stem", &config, Some(&path), &[]);
        match result {
            CommandResult::Success(output) => {
                assert!(output.contains("test"));
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_extension_placeholder() {
        let config = create_config(vec![("show_ext", "echo $e")]);
        let path = PathBuf::from("/tmp/test.txt");
        let result = execute_command("show_ext", &config, Some(&path), &[]);
        match result {
            CommandResult::Success(output) => {
                assert!(output.contains("txt"));
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_failed_command() {
        let config = create_config(vec![("bad_cmd", "exit 1")]);
        let result = execute_command("bad_cmd", &config, None, &[]);
        assert!(matches!(result, CommandResult::Error(_)));
    }
}
