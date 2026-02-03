//! Archive preview (zip, tar.gz)

use std::path::Path;

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::common::{
    format_size, get_border_style, truncate_entry_name, unix_timestamp_to_date, ARCHIVE_MAX_ENTRIES,
};

/// Archive entry information
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// File/directory name (full path within archive)
    pub name: String,
    /// Size in bytes (0 for directories)
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Last modified time (optional)
    pub modified: Option<String>,
}

impl ArchiveEntry {
    /// Sort archive entries: directories first, then alphabetically by name
    pub fn sort_entries(entries: &mut [ArchiveEntry]) {
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
    }
}

/// Archive preview content
pub struct ArchivePreview {
    /// Archive entries
    pub entries: Vec<ArchiveEntry>,
    /// Total uncompressed size
    pub total_size: u64,
    /// Number of files (not directories)
    pub file_count: usize,
    /// Scroll position
    pub scroll: usize,
}

impl ArchivePreview {
    /// Load archive preview from zip file
    pub fn load_zip(path: &Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut entries = Vec::new();
        let mut total_size = 0u64;
        let mut file_count = 0usize;

        for i in 0..archive.len().min(ARCHIVE_MAX_ENTRIES) {
            let entry = archive.by_index(i)?;
            let is_dir = entry.is_dir();
            let size = entry.size();
            let name = truncate_entry_name(entry.name().to_string());

            // Format modified time if available
            let modified = entry
                .last_modified()
                .map(|dt| format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day()));

            if !is_dir {
                total_size += size;
                file_count += 1;
            }

            entries.push(ArchiveEntry {
                name,
                size,
                is_dir,
                modified,
            });
        }

        // Sort entries: directories first, then files, both alphabetically
        ArchiveEntry::sort_entries(&mut entries);

        Ok(Self {
            entries,
            total_size,
            file_count,
            scroll: 0,
        })
    }

    /// Load archive preview from tar.gz file
    pub fn load_tar_gz(path: &Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let decompressed = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressed);

        let mut entries = Vec::new();
        let mut total_size = 0u64;
        let mut file_count = 0usize;

        for (i, entry_result) in archive.entries()?.enumerate() {
            if i >= ARCHIVE_MAX_ENTRIES {
                break;
            }

            let entry = entry_result?;
            let header = entry.header();
            let is_dir = header.entry_type().is_dir();
            let size = header.size().unwrap_or(0);
            let name = truncate_entry_name(entry.path()?.to_string_lossy().to_string());

            // Format modified time if available
            let modified = header
                .mtime()
                .ok()
                .map(|mtime| unix_timestamp_to_date(mtime as i64));

            if !is_dir {
                total_size += size;
                file_count += 1;
            }

            entries.push(ArchiveEntry {
                name,
                size,
                is_dir,
                modified,
            });
        }

        // Sort entries: directories first, then files, both alphabetically
        ArchiveEntry::sort_entries(&mut entries);

        Ok(Self {
            entries,
            total_size,
            file_count,
            scroll: 0,
        })
    }

    /// Get visible line count
    pub fn line_count(&self) -> usize {
        self.entries.len() + 2 // +2 for header lines
    }
}

/// Render archive preview
pub fn render_archive_preview(
    frame: &mut Frame,
    preview: &ArchivePreview,
    area: Rect,
    title: &str,
    focused: bool,
) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let separator = "─".repeat(area.width.saturating_sub(4) as usize);

    let mut lines: Vec<Line> = Vec::new();

    // Header: archive info
    lines.push(Line::from(vec![Span::styled(
        format!(
            "  {} files, {}",
            preview.file_count,
            format_size(preview.total_size)
        ),
        Style::default().fg(Color::Cyan),
    )]));

    lines.push(Line::from(vec![Span::styled(
        format!("  {}", separator),
        Style::default().fg(Color::DarkGray),
    )]));

    // Skip header lines in scroll calculation
    let header_lines = 2;
    let content_start = preview.scroll.saturating_sub(header_lines);

    // Entry list
    for entry in preview
        .entries
        .iter()
        .skip(content_start)
        .take(visible_height.saturating_sub(header_lines))
    {
        let (icon, color) = if entry.is_dir {
            ("\u{f07b}", Color::Blue) // Folder icon
        } else {
            ("\u{f016}", Color::White) // File icon
        };

        let size_str = if entry.is_dir {
            String::new()
        } else {
            format_size(entry.size)
        };

        let date_str = entry.modified.as_deref().unwrap_or("");

        // Calculate name display width
        let max_name_width = area.width.saturating_sub(24) as usize;
        let display_name = if entry.name.len() > max_name_width {
            format!("{}...", &entry.name[..max_name_width.saturating_sub(3)])
        } else {
            entry.name.clone()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", icon), Style::default().fg(color)),
            Span::styled(
                format!("{:<width$}", display_name, width = max_name_width),
                Style::default().fg(color),
            ),
            Span::styled(
                format!("{:>8}  ", size_str),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(date_str.to_string(), Style::default().fg(Color::DarkGray)),
        ]));
    }

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title))
            .border_style(get_border_style(focused)),
    );

    frame.render_widget(widget, area);
}

/// Check if a file is a tar.gz archive (handles double extension)
pub fn is_tar_gz_file(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz")
}

/// Check if a file is an archive (zip or tar.gz)
pub fn is_archive_file(path: &std::path::Path) -> bool {
    // Check tar.gz first (has double extension)
    if is_tar_gz_file(path) {
        return true;
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(
        ext.as_deref(),
        Some("zip" | "jar" | "apk" | "ipa" | "xpi" | "epub")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_archive_file_zip_variants() {
        assert!(is_archive_file(Path::new("file.zip")));
        assert!(is_archive_file(Path::new("library.jar")));
        assert!(is_archive_file(Path::new("app.apk")));
        assert!(is_archive_file(Path::new("app.ipa")));
        assert!(is_archive_file(Path::new("addon.xpi")));
        assert!(is_archive_file(Path::new("book.epub")));
    }

    #[test]
    fn test_is_archive_file_case_insensitive() {
        assert!(is_archive_file(Path::new("FILE.ZIP")));
        assert!(is_archive_file(Path::new("LIBRARY.JAR")));
        assert!(is_archive_file(Path::new("APP.APK")));
        assert!(is_archive_file(Path::new("File.Zip")));
        assert!(is_archive_file(Path::new("Library.Jar")));
    }

    #[test]
    fn test_is_archive_file_non_archive() {
        assert!(!is_archive_file(Path::new("file.txt")));
        assert!(!is_archive_file(Path::new("file.md")));
        assert!(!is_archive_file(Path::new("file.rs")));
        assert!(!is_archive_file(Path::new("image.png")));
        assert!(!is_archive_file(Path::new("image.jpg")));
        assert!(!is_archive_file(Path::new("file.7z")));
        assert!(!is_archive_file(Path::new("file.rar")));
        assert!(!is_archive_file(Path::new("file.tar")));
    }

    #[test]
    fn test_is_tar_gz_file() {
        assert!(is_tar_gz_file(Path::new("file.tar.gz")));
        assert!(is_tar_gz_file(Path::new("file.tgz")));
        assert!(is_tar_gz_file(Path::new("FILE.TAR.GZ")));
        assert!(is_tar_gz_file(Path::new("FILE.TGZ")));
        assert!(is_tar_gz_file(Path::new("File.Tar.Gz")));
    }

    #[test]
    fn test_is_tar_gz_file_is_archive() {
        assert!(is_archive_file(Path::new("file.tar.gz")));
        assert!(is_archive_file(Path::new("file.tgz")));
    }

    #[test]
    fn test_is_tar_gz_file_non_tar_gz() {
        assert!(!is_tar_gz_file(Path::new("file.tar")));
        assert!(!is_tar_gz_file(Path::new("file.gz")));
        assert!(!is_tar_gz_file(Path::new("file.zip")));
    }

    #[test]
    fn test_is_archive_file_no_extension() {
        assert!(!is_archive_file(Path::new("Makefile")));
        assert!(!is_archive_file(Path::new("README")));
    }

    #[test]
    fn test_archive_entry_struct_file() {
        let entry = ArchiveEntry {
            name: "src/main.rs".to_string(),
            size: 1024,
            is_dir: false,
            modified: Some("2024-01-15".to_string()),
        };

        assert_eq!(entry.name, "src/main.rs");
        assert_eq!(entry.size, 1024);
        assert!(!entry.is_dir);
        assert_eq!(entry.modified, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_archive_entry_struct_directory() {
        let entry = ArchiveEntry {
            name: "src/".to_string(),
            size: 0,
            is_dir: true,
            modified: None,
        };

        assert_eq!(entry.name, "src/");
        assert_eq!(entry.size, 0);
        assert!(entry.is_dir);
        assert!(entry.modified.is_none());
    }

    #[test]
    fn test_archive_preview_line_count() {
        let preview = ArchivePreview {
            entries: vec![
                ArchiveEntry {
                    name: "file1.txt".to_string(),
                    size: 100,
                    is_dir: false,
                    modified: None,
                },
                ArchiveEntry {
                    name: "file2.txt".to_string(),
                    size: 200,
                    is_dir: false,
                    modified: None,
                },
            ],
            total_size: 300,
            file_count: 2,
            scroll: 0,
        };

        assert_eq!(preview.line_count(), 4);
    }

    #[test]
    fn test_archive_preview_empty() {
        let preview = ArchivePreview {
            entries: vec![],
            total_size: 0,
            file_count: 0,
            scroll: 0,
        };

        assert_eq!(preview.line_count(), 2);
    }
}
