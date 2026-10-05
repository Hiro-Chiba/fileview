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
use super::window::{TextAnchor, TextWindow};

#[path = "text_syntax.rs"]
mod file_syntax;

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

/// Warm both lazy caches without doing real work.
///
/// First-call cost is roughly 20 to 50 ms for the syntax set and a few
/// hundred KB of heap for the embedded grammars and theme. Calling this
/// from a background thread at startup absorbs that cost so the first
/// preview, the first diff render, and the first Claude / context-pack
/// invocation do not pay it on the UI thread.
pub fn warmup() {
    let _ = get_syntax_set();
    let _ = get_theme();
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
#[derive(Clone)]
pub struct TextPreview {
    pub lines: Vec<String>,
    /// Syntax-highlighted lines (None for plain text)
    pub styled_lines: Option<Vec<StyledLine>>,
    pub scroll: usize,
    pub first_line: u64,
    pub anchor: Option<TextAnchor>,
    pub next: Option<TextAnchor>,
    pub prefix_clipped: bool,
    /// Visible reason when resource limits prevent accurate syntax replay.
    pub syntax_note: Option<&'static str>,
    syntax_complete: bool,
}

impl TextPreview {
    /// Build the immediately available page. Distant pages can be enriched by
    /// `highlight_with_context` after publishing this first response.
    pub fn from_window(
        window: TextWindow,
        path: &Path,
        cancelled: &dyn Fn() -> bool,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(!cancelled(), "preview cancelled");
        let styled_lines = if window.anchor.byte_offset == 0
            && !window.partial_last_line
            && window.text.lines().all(|line| line.len() <= 4096)
        {
            highlight_content_cancelled(&window.text, path, cancelled)
        } else {
            None
        };
        anyhow::ensure!(!cancelled(), "preview cancelled");
        Ok(Self {
            syntax_complete: styled_lines.is_some(),
            syntax_note: None,
            lines: window.text.lines().map(String::from).collect(),
            styled_lines,
            scroll: 0,
            first_line: window.first_line,
            anchor: Some(window.anchor),
            next: window.next,
            prefix_clipped: window.prefix_clipped,
        })
    }

    /// Whether a background pass can recover syntax context for this page.
    pub fn needs_context_highlighting(&self) -> bool {
        self.anchor.is_some() && !self.syntax_complete
    }

    /// Enrich a previously published page without changing its text or scroll.
    pub fn highlight_with_context(
        &mut self,
        path: &Path,
        cancelled: &dyn Fn() -> bool,
    ) -> anyhow::Result<()> {
        let Some(anchor) = self.anchor.as_ref() else {
            return Ok(());
        };
        let window = super::window::read_window(path, Some(anchor), cancelled)?;
        anyhow::ensure!(
            window
                .text
                .lines()
                .eq(self.lines.iter().map(String::as_str)),
            "preview content changed before syntax replay"
        );
        let result = file_syntax::highlight(path, &window, cancelled)?;
        anyhow::ensure!(!cancelled(), "preview cancelled");
        self.styled_lines = result.lines;
        self.syntax_note = result.note;
        self.syntax_complete = true;
        Ok(())
    }

    /// Create a new text preview without syntax highlighting
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        Self {
            lines,
            styled_lines: None,
            scroll: 0,
            first_line: 1,
            anchor: None,
            next: None,
            prefix_clipped: false,
            syntax_note: None,
            syntax_complete: true,
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
            first_line: 1,
            anchor: None,
            next: None,
            prefix_clipped: false,
            syntax_note: None,
            syntax_complete: true,
        }
    }
}

/// Perform syntax highlighting on content based on file extension
fn highlight_content(content: &str, path: &Path) -> Option<Vec<StyledLine>> {
    highlight_content_cancelled(content, path, &|| false)
}

fn highlight_content_cancelled(
    content: &str,
    path: &Path,
    cancelled: &dyn Fn() -> bool,
) -> Option<Vec<StyledLine>> {
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
        if cancelled() {
            return None;
        }
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
    let start = preview.scroll.min(preview.lines.len());
    let end = (start + visible_height).min(preview.lines.len());

    let lines: Vec<Line> = if let Some(ref styled_lines) = preview.styled_lines {
        // Render with syntax highlighting
        styled_lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, styled_line)| {
                let line_num = preview.first_line + (start + i) as u64;
                let mut spans = vec![Span::styled(
                    format!("{:4} ", line_num),
                    Style::default().fg(Color::DarkGray),
                )];
                if preview.prefix_clipped && start + i == 0 {
                    spans.push(Span::raw("… "));
                }
                for segment in &styled_line.segments {
                    spans.push(Span::styled(
                        segment.text.as_str(),
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
                let line_num = preview.first_line + (start + i) as u64;
                Line::from(vec![
                    Span::styled(
                        format!("{:4} ", line_num),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(if preview.prefix_clipped && start + i == 0 {
                        "… "
                    } else {
                        ""
                    }),
                    Span::raw(line.as_str()),
                ])
            })
            .collect()
    };

    let title = match preview.syntax_note {
        Some(note) => format!("{title} [{note}]"),
        None => title.to_owned(),
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
                | "log"
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

#[cfg(test)]
mod window_tests {
    use super::*;
    use crate::render::preview::window::{read_match_window, read_window, FileStamp};

    #[test]
    fn first_page_keeps_highlighting_but_distant_match_uses_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("code.rs");
        std::fs::write(&path, "fn main() {}\nlet 日本語 = 1;\n").unwrap();
        let first =
            TextPreview::from_window(read_window(&path, None, &|| false).unwrap(), &path, &|| {
                false
            })
            .unwrap();
        assert!(first.styled_lines.is_some());
        let anchor = TextAnchor {
            line_number: 2,
            byte_offset: 13,
            stamp: FileStamp::capture(&path).unwrap(),
        };
        let matched = read_match_window(&path, &anchor, 17, &|| false).unwrap();
        let preview = TextPreview::from_window(matched, &path, &|| false).unwrap();
        assert!(preview.styled_lines.is_none());
        assert_eq!(preview.first_line, 2);
        assert_eq!(preview.lines[0], "日本語 = 1;");
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(50, 5)).unwrap();
        terminal
            .draw(|frame| render_text_preview(frame, &preview, frame.area(), "code", true))
            .unwrap();
        let row = (0..50)
            .map(|x| terminal.backend().buffer()[(x, 1)].symbol())
            .collect::<String>();
        assert!(row.contains("   2 … "), "{row}");
        assert_eq!(terminal.backend().buffer()[(8, 1)].symbol(), "日");
        assert_eq!(terminal.backend().buffer()[(10, 1)].symbol(), "本");
        assert_eq!(terminal.backend().buffer()[(12, 1)].symbol(), "語");
    }

    #[test]
    fn cancelled_preview_does_not_publish_partly_highlighted_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("code.rs");
        std::fs::write(&path, "fn main() {}\n".repeat(256)).unwrap();
        let window = read_window(&path, None, &|| false).unwrap();
        let checks = std::cell::Cell::new(0);
        let result = TextPreview::from_window(window, &path, &|| {
            checks.set(checks.get() + 1);
            checks.get() > 5
        });
        assert!(result.is_err());
    }
    fn visible_colors(line: &StyledLine) -> Vec<(char, Color)> {
        line.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .text
                    .chars()
                    .filter(|ch| !matches!(ch, '\r' | '\n'))
                    .map(|ch| (ch, segment.color))
            })
            .collect()
    }

    #[test]
    fn distant_pages_match_whole_file_multiline_comments_and_strings() {
        for (extension, opening, closing) in [
            ("rs", "/* comment\r\n", "*/\r\nfn main() {}\r\n"),
            ("py", "text = \"\"\"string\r\n", "\"\"\"\r\nprint(text)\r\n"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(format!("code.{extension}"));
            let content = format!("{opening}{}{closing}", "日本語 inside\r\n".repeat(280));
            std::fs::write(&path, &content).unwrap();
            let first = read_window(&path, None, &|| false).unwrap();
            let window = read_window(&path, first.next.as_ref(), &|| false).unwrap();
            let mut preview = TextPreview::from_window(window, &path, &|| false).unwrap();
            assert!(preview.needs_context_highlighting());
            preview.highlight_with_context(&path, &|| false).unwrap();
            assert!(!preview.needs_context_highlighting());
            assert_eq!(preview.syntax_note, None);
            let whole = TextPreview::with_highlighting(&content, &path);
            let expected = &whole.styled_lines.as_ref().unwrap()[256..];
            let actual = preview.styled_lines.as_ref().unwrap();
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                assert_eq!(visible_colors(actual), visible_colors(expected));
            }
        }
    }

    #[test]
    fn match_inside_multiline_string_keeps_whole_line_syntax_and_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("code.py");
        let content = "value = \"\"\"start\r\n日本語 TARGET end\r\nfinish\"\"\"\r\n";
        std::fs::write(&path, content).unwrap();
        let line_start = content.find("日本語").unwrap();
        let matched = content.find("TARGET").unwrap();
        let anchor = TextAnchor {
            line_number: 2,
            byte_offset: line_start as u64,
            stamp: FileStamp::capture(&path).unwrap(),
        };
        let window = read_match_window(&path, &anchor, matched as u64, &|| false).unwrap();
        let mut preview = TextPreview::from_window(window, &path, &|| false).unwrap();
        preview.highlight_with_context(&path, &|| false).unwrap();
        let whole = TextPreview::with_highlighting(content, &path);
        let expected = visible_colors(&whole.styled_lines.as_ref().unwrap()[1]);
        assert_eq!(
            visible_colors(&preview.styled_lines.as_ref().unwrap()[0]),
            expected[4..]
        );
        assert!(preview.prefix_clipped);
    }

    #[test]
    fn oversized_context_stays_plain_with_a_visible_reason() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("code.rs");
        let content = format!("{}\nfn main() {{}}\n", "x".repeat(70_000));
        std::fs::write(&path, content).unwrap();
        let anchor = TextAnchor {
            line_number: 2,
            byte_offset: 70_001,
            stamp: FileStamp::capture(&path).unwrap(),
        };
        let window = read_window(&path, Some(&anchor), &|| false).unwrap();
        let mut preview = TextPreview::from_window(window, &path, &|| false).unwrap();
        preview.highlight_with_context(&path, &|| false).unwrap();
        assert!(preview.styled_lines.is_none());
        assert_eq!(preview.syntax_note, Some("plain: long line"));
    }

    #[test]
    fn context_replay_cancellation_does_not_publish_partial_styles() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("code.rs");
        std::fs::write(&path, "/*\n".to_owned() + &"comment\n".repeat(300) + "*/\n").unwrap();
        let first = read_window(&path, None, &|| false).unwrap();
        let window = read_window(&path, first.next.as_ref(), &|| false).unwrap();
        let mut preview = TextPreview::from_window(window, &path, &|| false).unwrap();
        let checks = std::cell::Cell::new(0);
        assert!(preview
            .highlight_with_context(&path, &|| {
                checks.set(checks.get() + 1);
                checks.get() > 30
            })
            .is_err());
        assert!(preview.styled_lines.is_none());
        assert!(preview.needs_context_highlighting());
    }
    #[test]
    fn truncated_last_line_uses_complete_line_syntax_before_clipping() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("code.rs");
        let content = format!(
            "/*\n{}{}*/\nfn main() {{}}\n",
            format!("{}\n", "a".repeat(299)).repeat(200),
            "x".repeat(10_000)
        );
        std::fs::write(&path, &content).unwrap();
        let window = read_window(&path, None, &|| false).unwrap();
        assert!(window.partial_last_line);
        let mut preview = TextPreview::from_window(window, &path, &|| false).unwrap();
        assert!(preview.styled_lines.is_none());
        preview.highlight_with_context(&path, &|| false).unwrap();
        let whole = TextPreview::with_highlighting(&content, &path);
        let actual = preview
            .styled_lines
            .as_ref()
            .expect("bounded comment replay");
        let last = actual.len() - 1;
        let expected = visible_colors(&whole.styled_lines.as_ref().unwrap()[last]);
        let shown = visible_colors(&actual[last]);
        assert_eq!(shown, expected[..shown.len()]);
    }

    #[test]
    fn changed_file_cannot_enrich_an_old_page() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("code.rs");
        std::fs::write(&path, "/*\ncomment\n*/\n").unwrap();
        let anchor = TextAnchor {
            line_number: 2,
            byte_offset: 3,
            stamp: FileStamp::capture(&path).unwrap(),
        };
        let mut preview = TextPreview::from_window(
            read_window(&path, Some(&anchor), &|| false).unwrap(),
            &path,
            &|| false,
        )
        .unwrap();
        std::fs::write(&path, "fn changed() {}\n").unwrap();
        assert!(preview.highlight_with_context(&path, &|| false).is_err());
        assert!(preview.styled_lines.is_none());
    }
    #[test]
    fn very_distant_page_remains_readable_when_replay_budget_is_exhausted() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("code.rs");
        let mut file = std::fs::File::create(&path).unwrap();
        let block = "//x\n".repeat(16_384);
        for _ in 0..129 {
            file.write_all(block.as_bytes()).unwrap();
        }
        file.write_all(b"fn target() {}\n").unwrap();
        drop(file);
        let anchor = TextAnchor {
            line_number: 16_384 * 129 + 1,
            byte_offset: 65_536 * 129,
            stamp: FileStamp::capture(&path).unwrap(),
        };
        let mut preview = TextPreview::from_window(
            read_window(&path, Some(&anchor), &|| false).unwrap(),
            &path,
            &|| false,
        )
        .unwrap();
        preview.highlight_with_context(&path, &|| false).unwrap();
        assert_eq!(preview.lines, ["fn target() {}"]);
        assert!(preview.styled_lines.is_none());
        assert_eq!(preview.syntax_note, Some("plain: syntax replay limit"));
        assert!(!preview.needs_context_highlighting());
    }
}
