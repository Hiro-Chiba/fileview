//! Small shared utilities.

use std::io;
use std::path::Path;

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
