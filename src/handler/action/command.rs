//! Custom command execution

use std::path::Path;
use std::process::Command;

use crate::app::CommandsConfig;

#[derive(Debug)]
pub enum CommandResult {
    Success(String),
    Error(String),
    /// Command not found in config
    NotFound,
}

/// `file_path` drives placeholder expansion, and `selected_paths` fills the `$S` placeholder.
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

    let cmd = if let Some(path) = file_path {
        CommandsConfig::expand_command(template, path, selected_paths)
    } else {
        template.clone()
    };

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
    fn test_selected_paths_with_quote_are_escaped() {
        let config = create_config(vec![("show_selected", "echo $S")]);
        let selected = vec![PathBuf::from("/tmp/it's.txt")];
        let result = execute_command(
            "show_selected",
            &config,
            Some(Path::new("/tmp/a")),
            &selected,
        );
        match result {
            CommandResult::Success(output) => {
                assert!(output.contains("it's.txt"));
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
