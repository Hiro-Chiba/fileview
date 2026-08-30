//! Cached file metadata and human-readable size/time formatters.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Get file size and modification time as a formatted string (full display)
/// Cached file metadata to avoid repeated stat() calls on the same path.
struct FileInfoCache {
    path: PathBuf,
    size: u64,
    is_dir: bool,
    modified: Option<SystemTime>,
}

thread_local! {
    static FILE_INFO_CACHE: RefCell<Option<FileInfoCache>> = const { RefCell::new(None) };
}

pub(crate) fn invalidate_file_info_cache() {
    FILE_INFO_CACHE.with(|cache| *cache.borrow_mut() = None);
}

/// Run `f` with cached metadata for `path`. Only calls stat() when the path changes.
fn with_cached_metadata<T>(path: &Path, f: impl FnOnce(&FileInfoCache) -> T) -> Option<T> {
    FILE_INFO_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let needs_update = cache.as_ref().is_none_or(|c| c.path != *path);
        if needs_update {
            let metadata = path.metadata().ok()?;
            *cache = Some(FileInfoCache {
                path: path.to_path_buf(),
                size: metadata.len(),
                is_dir: metadata.is_dir(),
                modified: metadata.modified().ok(),
            });
        }
        cache.as_ref().map(f)
    })
}

pub(super) fn get_file_info(path: &Path) -> Option<String> {
    with_cached_metadata(path, |c| {
        let size_str = if c.is_dir {
            "--".to_string()
        } else {
            format_size(c.size)
        };
        let mtime_str = c
            .modified
            .map(format_relative_time)
            .unwrap_or_else(|| "--".to_string());
        format!("{} · {}", size_str, mtime_str)
    })
}

/// Get file size and abbreviated modification time (narrow display)
pub(super) fn get_file_info_narrow(path: &Path) -> Option<String> {
    with_cached_metadata(path, |c| {
        let size_str = if c.is_dir {
            "--".to_string()
        } else {
            format_size(c.size)
        };
        let mtime_str = c
            .modified
            .map(format_relative_time_short)
            .unwrap_or_else(|| "--".to_string());
        format!("{} · {}", size_str, mtime_str)
    })
}

/// Get file size only (compact display)
pub(super) fn get_file_size_only(path: &Path) -> Option<String> {
    with_cached_metadata(path, |c| {
        if c.is_dir {
            "--".to_string()
        } else {
            format_size(c.size)
        }
    })
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format time as relative (e.g., "2h ago", "Yesterday", "Jan 30")
fn format_relative_time(time: SystemTime) -> String {
    let now = SystemTime::now();
    let duration = match now.duration_since(time) {
        Ok(d) => d,
        Err(_) => return "Future".to_string(),
    };

    let secs = duration.as_secs();
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    if secs < 60 {
        "Just now".to_string()
    } else if mins < 60 {
        format!("{}m ago", mins)
    } else if hours < 24 {
        format!("{}h ago", hours)
    } else if days == 1 {
        "Yesterday".to_string()
    } else if days < 7 {
        format!("{}d ago", days)
    } else {
        // Use date format for older files
        use std::time::UNIX_EPOCH;
        let timestamp = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format_date_from_timestamp(timestamp)
    }
}

/// Format time as short relative (e.g., "2m", "5h", "3d") for narrow displays
fn format_relative_time_short(time: SystemTime) -> String {
    let now = SystemTime::now();
    let duration = match now.duration_since(time) {
        Ok(d) => d,
        Err(_) => return "?".to_string(),
    };

    let secs = duration.as_secs();
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    if secs < 60 {
        "now".to_string()
    } else if mins < 60 {
        format!("{}m", mins)
    } else if hours < 24 {
        format!("{}h", hours)
    } else if days < 30 {
        format!("{}d", days)
    } else {
        // Use abbreviated date for older files
        use std::time::UNIX_EPOCH;
        let timestamp = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format_date_short_from_timestamp(timestamp)
    }
}

/// Format timestamp as "M/D" for narrow displays
fn format_date_short_from_timestamp(timestamp: u64) -> String {
    let secs_per_day: u64 = 86400;
    let days_since_epoch = timestamp / secs_per_day;

    let mut year = 1970u32;
    let mut remaining_days = days_since_epoch as u32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let months = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 1u32;
    let mut day = remaining_days + 1;

    for (i, &days) in months.iter().enumerate() {
        if remaining_days < days {
            month = (i + 1) as u32;
            day = remaining_days + 1;
            break;
        }
        remaining_days -= days;
    }

    format!("{}/{}", month, day)
}

/// Format timestamp as "Mon DD" or "Mon DD YYYY" if not current year
fn format_date_from_timestamp(timestamp: u64) -> String {
    // Simple month calculation (approximate, but good enough for display)
    let secs_per_day: u64 = 86400;
    let days_since_epoch = timestamp / secs_per_day;

    // Calculate year, month, day (simplified)
    let mut year = 1970u32;
    let mut remaining_days = days_since_epoch as u32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let months = [
        ("Jan", 31),
        ("Feb", if is_leap_year(year) { 29 } else { 28 }),
        ("Mar", 31),
        ("Apr", 30),
        ("May", 31),
        ("Jun", 30),
        ("Jul", 31),
        ("Aug", 31),
        ("Sep", 30),
        ("Oct", 31),
        ("Nov", 30),
        ("Dec", 31),
    ];

    let mut month_name = "Jan";
    let mut day = remaining_days + 1;

    for (name, days) in months.iter() {
        if remaining_days < *days {
            month_name = name;
            day = remaining_days + 1;
            break;
        }
        remaining_days -= days;
    }

    // Get current year for comparison
    let now_timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let current_year = 1970 + (now_timestamp / (365 * secs_per_day)) as u32;

    if year == current_year {
        format!("{} {}", month_name, day)
    } else {
        format!("{} {} {}", month_name, day, year)
    }
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
