//! Small shared utilities.

use std::io;
use std::path::Path;

/// Default cap for reading config/session/state files into memory (16 MiB).
///
/// These files live under the user's own config/cache/repo, so this is purely
/// defence-in-depth against a maliciously huge file causing unbounded memory
/// use during parsing rather than a hard security boundary.
pub const MAX_STATE_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Read a file to a string, refusing to read more than `max_bytes`.
///
/// Returns an `InvalidData` error if the file exceeds the cap, so callers that
/// already handle `io::Result` degrade gracefully instead of allocating an
/// arbitrarily large buffer.
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
}
