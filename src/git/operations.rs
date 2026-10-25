//! Git operations (stage, unstage)

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

static GIT_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Find Git using the operating system's normal PATH resolution.
pub(super) fn find_git_executable() -> Option<&'static Path> {
    let available = GIT_AVAILABLE.get_or_init(|| {
        Command::new("git")
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    });
    available.then(|| Path::new("git"))
}

pub fn stage(repo_root: &Path, file: &Path) -> anyhow::Result<()> {
    let git = find_git_executable().ok_or_else(|| anyhow::anyhow!("git not found"))?;

    let relative = file.strip_prefix(repo_root).unwrap_or(file);

    let output = Command::new(git)
        .args(["add", "--"])
        .arg(relative)
        .current_dir(repo_root)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git add failed: {}", stderr.trim())
    }
}

pub fn unstage(repo_root: &Path, file: &Path) -> anyhow::Result<()> {
    let git = find_git_executable().ok_or_else(|| anyhow::anyhow!("git not found"))?;

    let relative = file.strip_prefix(repo_root).unwrap_or(file);

    let output = Command::new(git)
        .args(["reset", "HEAD", "--"])
        .arg(relative)
        .current_dir(repo_root)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git reset failed: {}", stderr.trim())
    }
}

/// Returns false when the file is not staged or not in a git repo.
pub fn is_staged(repo_root: &Path, file: &Path) -> bool {
    let git = match find_git_executable() {
        Some(g) => g,
        None => return false,
    };

    let relative = file.strip_prefix(repo_root).unwrap_or(file);

    let output = Command::new(git)
        .args(["diff", "--cached", "--name-only", "--"])
        .arg(relative)
        .current_dir(repo_root)
        .output();

    match output {
        Ok(o) => {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                !stdout.trim().is_empty()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    fn init_git_repo(dir: &TempDir) -> bool {
        StdCommand::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn configure_git_user(dir: &TempDir) -> bool {
        let config_name = StdCommand::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir.path())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let config_email = StdCommand::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir.path())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        config_name && config_email
    }

    #[test]
    fn test_find_git_executable() {
        let command_available = StdCommand::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        if command_available {
            assert_eq!(find_git_executable(), Some(Path::new("git")));
        }
    }

    #[test]
    fn test_stage_nonexistent_repo() {
        let result = stage(
            Path::new("/nonexistent/repo"),
            Path::new("/nonexistent/file.txt"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_unstage_nonexistent_repo() {
        let result = unstage(
            Path::new("/nonexistent/repo"),
            Path::new("/nonexistent/file.txt"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_is_staged_nonexistent_repo() {
        let result = is_staged(
            Path::new("/nonexistent/repo"),
            Path::new("/nonexistent/file.txt"),
        );
        assert!(!result);
    }

    #[test]
    fn test_is_staged_not_in_repo() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        assert!(!is_staged(temp.path(), &file));
    }

    #[test]
    fn test_stage_in_real_repo() {
        let temp = TempDir::new().unwrap();

        if find_git_executable().is_none() {
            return; // Skip if git not available
        }

        if !init_git_repo(&temp) || !configure_git_user(&temp) {
            return; // Skip if git init fails
        }

        let file = temp.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        let result = stage(temp.path(), &file);
        assert!(result.is_ok());

        assert!(is_staged(temp.path(), &file));
    }

    #[test]
    fn test_unstage_in_real_repo() {
        let temp = TempDir::new().unwrap();

        if find_git_executable().is_none() {
            return;
        }

        if !init_git_repo(&temp) || !configure_git_user(&temp) {
            return;
        }

        let file = temp.path().join("test.txt");
        fs::write(&file, "content").unwrap();

        stage(temp.path(), &file).unwrap();
        assert!(is_staged(temp.path(), &file));

        let result = unstage(temp.path(), &file);
        assert!(result.is_ok());

        assert!(!is_staged(temp.path(), &file));
    }

    #[test]
    fn test_stage_relative_path() {
        let temp = TempDir::new().unwrap();

        if find_git_executable().is_none() {
            return;
        }

        if !init_git_repo(&temp) || !configure_git_user(&temp) {
            return;
        }

        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let file = subdir.join("test.txt");
        fs::write(&file, "content").unwrap();

        let result = stage(temp.path(), &file);
        assert!(result.is_ok());
        assert!(is_staged(temp.path(), &file));
    }

    #[test]
    fn test_is_staged_untracked_file() {
        let temp = TempDir::new().unwrap();

        if find_git_executable().is_none() {
            return;
        }

        if !init_git_repo(&temp) {
            return;
        }

        let file = temp.path().join("untracked.txt");
        fs::write(&file, "content").unwrap();

        assert!(!is_staged(temp.path(), &file));
    }
}
