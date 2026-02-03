//! Common utilities for preview rendering

use ratatui::style::{Color, Style};

/// Maximum depth for recursive directory size calculation (for performance)
pub const MAX_DIR_SIZE_DEPTH: u32 = 3;

/// Maximum bytes to read for hex preview
pub const HEX_PREVIEW_MAX_BYTES: usize = 4096;

/// Number of bytes per line in hex preview
pub const HEX_BYTES_PER_LINE: usize = 16;

/// Maximum entries to display in archive preview
pub const ARCHIVE_MAX_ENTRIES: usize = 500;

/// Maximum length for archive entry names (prevent DoS from malicious archives)
pub const MAX_ENTRY_NAME_LEN: usize = 4096;

/// Get border style based on focus state
pub fn get_border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}

/// Truncate archive entry name if too long
pub fn truncate_entry_name(name: String) -> String {
    if name.len() > MAX_ENTRY_NAME_LEN {
        format!("{}...", &name[..MAX_ENTRY_NAME_LEN - 3])
    } else {
        name
    }
}

/// Format bytes as human-readable string
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Convert Unix timestamp to date string (YYYY-MM-DD)
pub fn unix_timestamp_to_date(secs: i64) -> String {
    const SECONDS_PER_DAY: i64 = 86400;
    const DAYS_IN_MONTH: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut days = secs / SECONDS_PER_DAY;
    let mut year = 1970i64;

    // Find year
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    // Find month and day
    let leap = is_leap_year(year);
    let mut month = 1;
    for (i, &d) in DAYS_IN_MONTH.iter().enumerate() {
        let days_in_month = if i == 1 && leap { 29 } else { d };
        if days < days_in_month {
            break;
        }
        days -= days_in_month;
        month += 1;
    }
    let day = days + 1;

    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Check if a year is a leap year
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Calculate total size of a directory (recursive, with depth limit)
pub fn calculate_dir_size(path: &std::path::Path) -> anyhow::Result<u64> {
    calculate_dir_size_recursive(path, 0, MAX_DIR_SIZE_DEPTH as usize)
}

fn calculate_dir_size_recursive(
    path: &std::path::Path,
    depth: usize,
    max_depth: usize,
) -> anyhow::Result<u64> {
    if depth > max_depth {
        return Ok(0);
    }

    let mut total = 0u64;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total += metadata.len();
                } else if metadata.is_dir() {
                    if let Ok(sub_size) =
                        calculate_dir_size_recursive(&entry.path(), depth + 1, max_depth)
                    {
                        total += sub_size;
                    }
                }
            }
        }
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(100), "100 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kb_mb_gb() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 1024), "1.0 TB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(2 * 1024 * 1024 + 512 * 1024), "2.5 MB");
    }
}
