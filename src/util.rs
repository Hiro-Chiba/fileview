//! Small shared utilities.

use std::io;
use std::path::{Path, PathBuf};

/// Find a preview executable on PATH, with common Unix installation paths as a fallback.
/// Windows executables use `.exe`; no external `which` command is required.
pub(crate) fn find_preview_tool(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let directories = std::env::split_paths(&path);
    #[cfg(unix)]
    let directories =
        directories.chain(["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"].map(PathBuf::from));
    find_preview_tool_in(name, directories)
}

fn find_preview_tool_in(
    name: &str,
    directories: impl IntoIterator<Item = PathBuf>,
) -> Option<PathBuf> {
    let filename = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    directories.into_iter().find_map(|directory| {
        let candidate = directory.join(&filename);
        let metadata = candidate.metadata().ok()?;
        if !metadata.is_file() {
            return None;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                return None;
            }
        }
        Some(candidate)
    })
}

/// Cap for reading config/session/state files into memory (16 MiB).
/// Defence-in-depth against an oversized file exhausting memory during parsing.
pub const MAX_STATE_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Return at most `max_bytes` from the start of a string without splitting a
/// UTF-8 code point.
pub fn utf8_prefix(value: &str, max_bytes: usize) -> &str {
    let mut end = max_bytes.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

/// Return at most `max_bytes` from the end of a string without splitting a
/// UTF-8 code point.
pub fn utf8_suffix(value: &str, max_bytes: usize) -> &str {
    let mut start = value.len().saturating_sub(max_bytes);
    while !value.is_char_boundary(start) {
        start += 1;
    }
    &value[start..]
}

/// Read a file to a string, returning `InvalidData` if it exceeds `max_bytes`.
pub fn read_to_string_capped(path: &Path, max_bytes: u64) -> io::Result<String> {
    let len = std::fs::metadata(path)?.len();
    if len > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "file {} is {} bytes, exceeding the {}-byte limit",
                path.display(),
                len,
                max_bytes
            ),
        ));
    }
    std::fs::read_to_string(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn preview_tool_lookup_uses_platform_filename_and_path_order() {
        let temp = tempdir().unwrap();
        let first = temp.path().join("first bin");
        let second = temp.path().join("second bin");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        let filename = format!("ffprobe{}", std::env::consts::EXE_SUFFIX);
        for directory in [&first, &second] {
            let executable = directory.join(&filename);
            fs::write(&executable, "test executable").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        assert_eq!(
            find_preview_tool_in("ffprobe", [first.clone(), second]),
            Some(first.join(filename))
        );
    }

    #[test]
    fn preview_tool_lookup_skips_missing_files_and_directories() {
        let temp = tempdir().unwrap();
        let filename = format!("ffprobe{}", std::env::consts::EXE_SUFFIX);
        fs::create_dir(temp.path().join(filename)).unwrap();
        assert!(find_preview_tool_in(
            "ffprobe",
            [temp.path().join("missing"), temp.path().to_path_buf()]
        )
        .is_none());
    }

    #[cfg(unix)]
    #[test]
    fn preview_tool_lookup_skips_non_executable_files() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempdir().unwrap();
        let executable = temp.path().join("ffprobe");
        fs::write(&executable, "not executable").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(find_preview_tool_in("ffprobe", [temp.path().to_path_buf()]).is_none());
    }

    #[test]
    fn reads_small_file() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("ok.txt");
        fs::write(&p, "hello").unwrap();
        assert_eq!(
            read_to_string_capped(&p, MAX_STATE_FILE_BYTES).unwrap(),
            "hello"
        );
    }

    #[test]
    fn rejects_oversized_file() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("big.txt");
        fs::write(&p, "0123456789").unwrap();
        assert!(read_to_string_capped(&p, 5).is_err());
    }

    #[test]
    fn utf8_slices_stop_at_character_boundaries() {
        assert_eq!(utf8_prefix("日本abc", 4), "日");
        assert_eq!(utf8_prefix("日本abc", 6), "日本");
        assert_eq!(utf8_suffix("abc日本", 4), "本");
        assert_eq!(utf8_suffix("abc日本", 6), "日本");
        assert_eq!(utf8_prefix("日", 0), "");
        assert_eq!(utf8_suffix("日", 0), "");
    }
}
