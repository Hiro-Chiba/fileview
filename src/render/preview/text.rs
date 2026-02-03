//! Text preview with syntax highlighting

use std::path::Path;
use std::sync::OnceLock;

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use super::common::get_border_style;

/// Lazy-initialized syntax set (100+ languages)
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();

/// Lazy-initialized theme (base16-ocean.dark)
static THEME: OnceLock<Theme> = OnceLock::new();

/// Get the shared syntax set (lazy-initialized)
fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Get the shared theme (lazy-initialized)
fn get_theme() -> &'static Theme {
    THEME.get_or_init(|| {
        let ts = ThemeSet::load_defaults();
        ts.themes["base16-ocean.dark"].clone()
    })
}

/// A segment of styled text (text with color)
#[derive(Debug, Clone)]
pub struct StyledSegment {
    pub text: String,
    pub color: Color,
}

/// A line with syntax highlighting
#[derive(Debug, Clone)]
pub struct StyledLine {
    pub segments: Vec<StyledSegment>,
}

/// Text preview content
pub struct TextPreview {
    pub lines: Vec<String>,
    /// Syntax-highlighted lines (None for plain text)
    pub styled_lines: Option<Vec<StyledLine>>,
    pub scroll: usize,
}

impl TextPreview {
    /// Create a new text preview without syntax highlighting
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        Self {
            lines,
            styled_lines: None,
            scroll: 0,
        }
    }

    /// Create a new text preview with syntax highlighting based on file extension
    pub fn with_highlighting(content: &str, path: &Path) -> Self {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        let styled_lines = highlight_content(content, path);
        Self {
            lines,
            styled_lines,
            scroll: 0,
        }
    }
}

/// Perform syntax highlighting on content based on file extension
fn highlight_content(content: &str, path: &Path) -> Option<Vec<StyledLine>> {
    let ss = get_syntax_set();
    let theme = get_theme();

    // Detect syntax from file extension or first line (shebang)
    let syntax = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(|ext| ss.find_syntax_by_extension(ext))
        .or_else(|| ss.find_syntax_by_first_line(content.lines().next().unwrap_or("")))?;

    let mut h = HighlightLines::new(syntax, theme);
    let mut styled_lines = Vec::new();

    for line in LinesWithEndings::from(content) {
        let ranges = h.highlight_line(line, ss).ok()?;
        let segments = ranges
            .iter()
            .map(|(style, text)| {
                let color = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                StyledSegment {
                    text: text.to_string(),
                    color,
                }
            })
            .collect();
        styled_lines.push(StyledLine { segments });
    }

    Some(styled_lines)
}

/// Render text preview
pub fn render_text_preview(
    frame: &mut Frame,
    preview: &TextPreview,
    area: Rect,
    title: &str,
    focused: bool,
) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let start = preview.scroll;
    let end = (start + visible_height).min(preview.lines.len());

    let lines: Vec<Line> = if let Some(ref styled_lines) = preview.styled_lines {
        // Render with syntax highlighting
        styled_lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, styled_line)| {
                let line_num = start + i + 1;
                let mut spans = vec![Span::styled(
                    format!("{:4} ", line_num),
                    Style::default().fg(Color::DarkGray),
                )];
                for segment in &styled_line.segments {
                    spans.push(Span::styled(
                        segment.text.clone(),
                        Style::default().fg(segment.color),
                    ));
                }
                Line::from(spans)
            })
            .collect()
    } else {
        // Render plain text (fallback)
        preview.lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = start + i + 1;
                Line::from(vec![
                    Span::styled(
                        format!("{:4} ", line_num),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(line.as_str()),
                ])
            })
            .collect()
    };

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title))
            .border_style(get_border_style(focused)),
    );

    frame.render_widget(widget, area);
}

/// Check if a file is likely a text file
pub fn is_text_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(
        ext.as_deref(),
        Some(
            "txt"
                | "md"
                | "rs"
                | "py"
                | "js"
                | "ts"
                | "jsx"
                | "tsx"
                | "html"
                | "css"
                | "json"
                | "toml"
                | "yaml"
                | "yml"
                | "xml"
                | "sh"
                | "bash"
                | "zsh"
                | "c"
                | "h"
                | "cpp"
                | "hpp"
                | "java"
                | "go"
                | "rb"
                | "php"
                | "sql"
                | "vim"
                | "lua"
                | "el"
                | "lisp"
                | "scm"
                | "hs"
                | "ml"
                | "ex"
                | "exs"
                | "erl"
                | "clj"
                | "swift"
                | "kt"
                | "scala"
                | "r"
                | "jl"
                | "pl"
                | "pm"
                | "awk"
                | "sed"
                | "conf"
                | "cfg"
                | "ini"
                | "env"
                | "gitignore"
                | "dockerignore"
                | "makefile"
                | "cmake"
        )
    )
}
