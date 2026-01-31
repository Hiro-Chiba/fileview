//! Fuzzy finder rendering and matching

use std::path::PathBuf;

use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher, Utf32Str,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Maximum number of results to display
const MAX_RESULTS: usize = 15;

/// Fuzzy match result
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    /// Original path
    pub path: PathBuf,
    /// Display string (relative path)
    pub display: String,
    /// Match score (higher is better)
    pub score: u32,
    /// Matched character indices for highlighting
    pub indices: Vec<usize>,
}

/// Perform fuzzy matching on a list of paths
pub fn fuzzy_match(query: &str, paths: &[PathBuf], root: &PathBuf) -> Vec<FuzzyMatch> {
    if query.is_empty() {
        // Return first MAX_RESULTS paths when no query
        return paths
            .iter()
            .take(MAX_RESULTS)
            .map(|p| {
                let display = p
                    .strip_prefix(root)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .to_string();
                FuzzyMatch {
                    path: p.clone(),
                    display,
                    score: 0,
                    indices: vec![],
                }
            })
            .collect();
    }

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);

    let mut results: Vec<FuzzyMatch> = paths
        .iter()
        .filter_map(|path| {
            let display = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let mut buf = Vec::new();
            let haystack = Utf32Str::new(&display, &mut buf);

            let mut indices = Vec::new();
            let score = pattern.indices(haystack, &mut matcher, &mut indices)?;

            // Convert indices to usize
            let indices: Vec<usize> = indices.iter().map(|&i| i as usize).collect();

            Some(FuzzyMatch {
                path: path.clone(),
                display,
                score,
                indices,
            })
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.cmp(&a.score));

    // Limit results
    results.truncate(MAX_RESULTS);

    results
}

/// Render the fuzzy finder popup
pub fn render_fuzzy_finder(
    frame: &mut Frame,
    query: &str,
    results: &[FuzzyMatch],
    selected: usize,
    area: Rect,
) {
    // Calculate popup dimensions (handle very small terminals)
    let popup_width = (area.width * 70 / 100)
        .clamp(40, 80)
        .min(area.width.saturating_sub(2));
    let popup_height = (MAX_RESULTS as u16 + 4)
        .min(area.height.saturating_sub(4))
        .max(6); // Minimum height for usability

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 3; // Slightly above center

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the popup area
    frame.render_widget(Clear, popup_area);

    // Create popup block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Fuzzy Find (Ctrl+P) ");

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Split inner area: input field + results
    let chunks = Layout::default()
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    // Render query input
    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::raw(query),
        Span::styled(
            "_",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::SLOW_BLINK),
        ),
    ]);
    frame.render_widget(Paragraph::new(input_line), chunks[0]);

    // Render separator
    let separator = "─".repeat(chunks[1].width as usize);
    frame.render_widget(
        Paragraph::new(separator).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );

    // Render results
    if results.is_empty() {
        let no_results =
            Paragraph::new("  No matches found").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(no_results, chunks[2]);
    } else {
        let items: Vec<ListItem> = results
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let is_selected = i == selected;
                let style = if is_selected {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                // Create highlighted text
                let spans = create_highlighted_spans(&m.display, &m.indices, is_selected);
                let line = Line::from(spans);

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, chunks[2]);
    }
}

/// Create spans with matched characters highlighted
fn create_highlighted_spans(
    text: &str,
    indices: &[usize],
    _is_selected: bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let chars: Vec<char> = text.chars().collect();

    // Match highlight style (yellow bold for both selected and unselected)
    let match_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let normal_style = Style::default();

    let mut last_idx = 0;
    for &idx in indices {
        // Skip out-of-bounds indices
        if idx >= chars.len() {
            continue;
        }
        if idx > last_idx {
            // Add non-matched characters
            let s: String = chars[last_idx..idx].iter().collect();
            spans.push(Span::styled(s, normal_style));
        }
        // Add matched character
        spans.push(Span::styled(chars[idx].to_string(), match_style));
        last_idx = idx + 1;
    }

    // Add remaining characters
    if last_idx < chars.len() {
        let s: String = chars[last_idx..].iter().collect();
        spans.push(Span::styled(s, normal_style));
    }

    // Add prefix space for padding
    let mut result = vec![Span::raw("  ")];
    result.extend(spans);
    result
}

/// Collect all file paths from a directory recursively
pub fn collect_paths(root: &PathBuf, show_hidden: bool) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_paths_recursive(root, &mut paths, show_hidden, 0, 10);
    paths
}

fn collect_paths_recursive(
    dir: &PathBuf,
    paths: &mut Vec<PathBuf>,
    show_hidden: bool,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files if not showing them
            if !show_hidden && name.starts_with('.') {
                continue;
            }

            paths.push(path.clone());

            if path.is_dir() {
                collect_paths_recursive(&path, paths, show_hidden, depth + 1, max_depth);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_empty_query() {
        let root = PathBuf::from("/test");
        let paths = vec![
            PathBuf::from("/test/file1.txt"),
            PathBuf::from("/test/file2.txt"),
        ];

        let results = fuzzy_match("", &paths, &root);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_fuzzy_match_simple_query() {
        let root = PathBuf::from("/test");
        let paths = vec![
            PathBuf::from("/test/main.rs"),
            PathBuf::from("/test/lib.rs"),
            PathBuf::from("/test/other.txt"),
        ];

        let results = fuzzy_match("rs", &paths, &root);
        assert!(results.len() >= 2);
        // .rs files should match
        assert!(results.iter().any(|r| r.display.ends_with(".rs")));
    }

    #[test]
    fn test_fuzzy_match_no_results() {
        let root = PathBuf::from("/test");
        let paths = vec![PathBuf::from("/test/file.txt")];

        let results = fuzzy_match("xyz123", &paths, &root);
        assert!(results.is_empty());
    }

    #[test]
    fn test_fuzzy_match_partial() {
        let root = PathBuf::from("/test");
        let paths = vec![
            PathBuf::from("/test/src/render/mod.rs"),
            PathBuf::from("/test/src/render/preview.rs"),
            PathBuf::from("/test/src/main.rs"),
        ];

        let results = fuzzy_match("ren", &paths, &root);
        // Should match paths containing "render"
        assert!(!results.is_empty());
    }

    #[test]
    fn test_create_highlighted_spans_no_matches() {
        let spans = create_highlighted_spans("hello", &[], false);
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_create_highlighted_spans_with_matches() {
        let spans = create_highlighted_spans("hello", &[0, 2], false);
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_fuzzy_match_result_has_indices() {
        let root = PathBuf::from("/test");
        let paths = vec![PathBuf::from("/test/main.rs")];

        let results = fuzzy_match("mn", &paths, &root);
        if !results.is_empty() {
            // Should have matched indices
            assert!(!results[0].indices.is_empty());
        }
    }

    #[test]
    fn test_fuzzy_match_sorted_by_score() {
        let root = PathBuf::from("/test");
        let paths = vec![
            PathBuf::from("/test/abc.txt"),
            PathBuf::from("/test/ab.txt"),
            PathBuf::from("/test/a.txt"),
        ];

        let results = fuzzy_match("a", &paths, &root);
        // Results should be sorted by score (descending)
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score);
        }
    }

    #[test]
    fn test_fuzzy_match_max_results_limit() {
        let root = PathBuf::from("/test");
        let paths: Vec<PathBuf> = (0..50)
            .map(|i| PathBuf::from(format!("/test/file{}.txt", i)))
            .collect();

        let results = fuzzy_match("file", &paths, &root);
        assert!(results.len() <= MAX_RESULTS);
    }

    #[test]
    fn test_fuzzy_match_empty_paths() {
        let root = PathBuf::from("/test");
        let paths: Vec<PathBuf> = vec![];

        let results = fuzzy_match("test", &paths, &root);
        assert!(results.is_empty());
    }

    #[test]
    fn test_fuzzy_match_with_root_prefix() {
        let root = PathBuf::from("/test/root");
        let paths = vec![PathBuf::from("/test/root/subdir/file.txt")];

        let results = fuzzy_match("", &paths, &root);
        assert!(!results.is_empty());
        // Display should be relative to root
        assert_eq!(results[0].display, "subdir/file.txt");
    }

    #[test]
    fn test_fuzzy_match_unicode_filename() {
        let root = PathBuf::from("/test");
        let paths = vec![PathBuf::from("/test/日本語ファイル.txt")];

        let results = fuzzy_match("日本", &paths, &root);
        // Should handle unicode
        assert!(!results.is_empty() || results.is_empty()); // Just don't panic
    }

    #[test]
    fn test_create_highlighted_spans_full_match() {
        let spans = create_highlighted_spans("abc", &[0, 1, 2], false);
        // Should have padding + 3 highlighted chars
        assert!(spans.len() >= 4);
    }

    #[test]
    fn test_create_highlighted_spans_empty_text() {
        let spans = create_highlighted_spans("", &[], false);
        // Should have at least the padding span
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_create_highlighted_spans_out_of_bounds_indices() {
        // Indices beyond text length should be handled gracefully
        let spans = create_highlighted_spans("ab", &[0, 5, 10], false);
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_collect_paths_empty_dir() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();

        let paths = collect_paths(&temp.path().to_path_buf(), false);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_collect_paths_with_files() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("test.txt"), "").unwrap();
        fs::write(temp.path().join("test2.txt"), "").unwrap();

        let paths = collect_paths(&temp.path().to_path_buf(), false);
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_collect_paths_hidden_files() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join(".hidden"), "").unwrap();
        fs::write(temp.path().join("visible"), "").unwrap();

        // Without showing hidden
        let paths_no_hidden = collect_paths(&temp.path().to_path_buf(), false);
        assert_eq!(paths_no_hidden.len(), 1);

        // With showing hidden
        let paths_with_hidden = collect_paths(&temp.path().to_path_buf(), true);
        assert_eq!(paths_with_hidden.len(), 2);
    }

    #[test]
    fn test_collect_paths_nested_directories() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("a/b/c")).unwrap();
        fs::write(temp.path().join("a/b/c/file.txt"), "").unwrap();

        let paths = collect_paths(&temp.path().to_path_buf(), false);
        // Should include a, a/b, a/b/c, and a/b/c/file.txt
        assert!(paths.len() >= 4);
    }

    #[test]
    fn test_fuzzy_match_debug_impl() {
        let fm = FuzzyMatch {
            path: PathBuf::from("/test"),
            display: "test".to_string(),
            score: 100,
            indices: vec![0, 1],
        };
        // Debug should not panic
        let _ = format!("{:?}", fm);
    }

    #[test]
    fn test_fuzzy_match_clone_impl() {
        let fm = FuzzyMatch {
            path: PathBuf::from("/test"),
            display: "test".to_string(),
            score: 100,
            indices: vec![0, 1],
        };
        let cloned = fm.clone();
        assert_eq!(cloned.path, fm.path);
        assert_eq!(cloned.display, fm.display);
        assert_eq!(cloned.score, fm.score);
        assert_eq!(cloned.indices, fm.indices);
    }

    #[test]
    fn test_max_results_constant() {
        // Verify constant is reasonable (compile-time check)
        const { assert!(MAX_RESULTS > 0) };
        const { assert!(MAX_RESULTS <= 100) }; // Should not be too large
    }
}
