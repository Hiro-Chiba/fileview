//! Rendered Markdown view for text previews on wide panes
//!
//! Rendering is line based so that every output line maps back to one source
//! line. A full CommonMark parser was not used because the existing scroll and
//! paging logic counts source lines, and keeping that mapping lets the
//! rendered view reuse it unchanged without adding a dependency.

use std::path::Path;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Preview panes at least this wide (including borders) show rendered Markdown.
pub const MIN_RENDER_WIDTH: u16 = 80;

/// Longer lines are shown as written. Inline parsing looks ahead for closing
/// markers, so its cost grows with the square of the line length, and a page
/// may hold a single 64 KiB line.
const MAX_INLINE_BYTES: usize = 1024;

/// Check if a path looks like a Markdown document
pub fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .is_some_and(|e| matches!(e.as_str(), "md" | "markdown" | "mdown" | "mkd"))
}

/// Render at most `max` output lines starting at source line `from`, pairing
/// each output line with its source line index.
///
/// Earlier lines are only scanned for code fences, so the cost of a frame
/// depends on the visible lines. Fence delimiters produce no output, so a
/// source index may be skipped.
pub fn render(
    lines: &[String],
    width: u16,
    from: usize,
    max: usize,
) -> Vec<(usize, Line<'static>)> {
    let mut out = Vec::with_capacity(max.min(lines.len()));
    let mut fence: Option<&str> = None;

    for (i, line) in lines.iter().enumerate() {
        if out.len() >= max {
            break;
        }
        let trimmed = line.trim_start();

        if let Some(marker) = fence {
            if trimmed.starts_with(marker)
                && trimmed
                    .trim_matches(|c| c == '`' || c == '~')
                    .trim()
                    .is_empty()
            {
                fence = None;
            } else if i >= from {
                out.push((
                    i,
                    Line::from(Span::styled(format!("  {line}"), code_style())),
                ));
            }
            continue;
        }

        if let Some(marker) = fence_marker(trimmed) {
            fence = Some(marker);
            continue;
        }

        if i >= from {
            out.push((i, render_block_line(line, width)));
        }
    }

    out
}

fn fence_marker(trimmed: &str) -> Option<&'static str> {
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn render_block_line(line: &str, width: u16) -> Line<'static> {
    if line.len() > MAX_INLINE_BYTES {
        return Line::raw(line.to_string());
    }
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    if trimmed.is_empty() {
        return Line::default();
    }

    if let Some((level, text)) = heading(trimmed) {
        return Line::from(inline(text, heading_style(level)));
    }

    if is_rule(trimmed) {
        let len = width.saturating_sub(2) as usize;
        return Line::from(Span::styled("─".repeat(len), dim_style()));
    }

    if let Some(rest) = trimmed.strip_prefix('>') {
        let mut spans = vec![Span::styled(format!("{indent}│ "), dim_style())];
        spans.extend(inline(
            rest.strip_prefix(' ').unwrap_or(rest),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        ));
        return Line::from(spans);
    }

    if let Some(rest) = bullet_item(trimmed) {
        let (marker, rest) = task_marker(rest);
        let mut spans = vec![Span::styled(
            format!("{indent}{marker}"),
            Style::default().fg(Color::Cyan),
        )];
        spans.extend(inline(rest, Style::default()));
        return Line::from(spans);
    }

    if let Some((number, rest)) = ordered_item(trimmed) {
        let mut spans = vec![Span::styled(
            format!("{indent}{number} "),
            Style::default().fg(Color::Cyan),
        )];
        spans.extend(inline(rest, Style::default()));
        return Line::from(spans);
    }

    if trimmed.starts_with('|') && is_table_separator(trimmed) {
        return Line::from(Span::styled(line.to_string(), dim_style()));
    }

    let mut spans = vec![Span::raw(indent.to_string())];
    spans.extend(inline(trimmed, Style::default()));
    Line::from(spans)
}

fn heading(trimmed: &str) -> Option<(usize, &str)> {
    let level = trimmed.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    Some((level, rest.trim().trim_end_matches('#').trim_end()))
}

fn is_rule(trimmed: &str) -> bool {
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 3
        && ['-', '*', '_']
            .iter()
            .any(|&m| compact.chars().all(|c| c == m))
}

fn bullet_item(trimmed: &str) -> Option<&str> {
    let mut chars = trimmed.chars();
    let marker = chars.next()?;
    if matches!(marker, '-' | '*' | '+') && chars.next() == Some(' ') {
        Some(&trimmed[2..])
    } else {
        None
    }
}

fn task_marker(rest: &str) -> (&'static str, &str) {
    if let Some(r) = rest.strip_prefix("[ ] ") {
        ("☐ ", r)
    } else if let Some(r) = rest
        .strip_prefix("[x] ")
        .or_else(|| rest.strip_prefix("[X] "))
    {
        ("☑ ", r)
    } else {
        ("• ", rest)
    }
}

fn ordered_item(trimmed: &str) -> Option<(&str, &str)> {
    let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let after = &trimmed[digits..];
    let mut chars = after.chars();
    if matches!(chars.next(), Some('.' | ')')) && chars.next() == Some(' ') {
        Some((&trimmed[..digits + 1], &trimmed[digits + 2..]))
    } else {
        None
    }
}

fn is_table_separator(trimmed: &str) -> bool {
    trimmed.contains('-') && trimmed.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

fn heading_style(level: usize) -> Style {
    let style = Style::default().add_modifier(Modifier::BOLD);
    match level {
        1 => style.fg(Color::Magenta).add_modifier(Modifier::UNDERLINED),
        2 => style.fg(Color::Magenta),
        _ => style.fg(Color::Blue),
    }
}

fn code_style() -> Style {
    Style::default().fg(Color::Yellow)
}

fn dim_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Render inline emphasis, code spans and links on top of a base style.
fn inline(text: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let mut bold = false;
    let mut italic = false;
    let mut strike = false;
    let current = |bold: bool, italic: bool, strike: bool| {
        let mut s = base;
        if bold {
            s = s.add_modifier(Modifier::BOLD);
        }
        if italic {
            s = s.add_modifier(Modifier::ITALIC);
        }
        if strike {
            s = s.add_modifier(Modifier::CROSSED_OUT);
        }
        s
    };
    let flush = |buf: &mut String, spans: &mut Vec<Span<'static>>, style: Style| {
        if !buf.is_empty() {
            spans.push(Span::styled(std::mem::take(buf), style));
        }
    };

    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let style = current(bold, italic, strike);

        if let Some(escaped) = rest.strip_prefix('\\').and_then(|r| r.chars().next()) {
            if escaped.is_ascii_punctuation() {
                buf.push(escaped);
                i += 1 + escaped.len_utf8();
                continue;
            }
        }

        if rest.starts_with('`') {
            let ticks = rest.bytes().take_while(|&b| b == b'`').count();
            let delim = &rest[..ticks];
            if let Some(end) = rest[ticks..].find(delim) {
                flush(&mut buf, &mut spans, style);
                let code = &rest[ticks..ticks + end];
                spans.push(Span::styled(code.trim().to_string(), code_style()));
                i += ticks * 2 + end;
                continue;
            }
        }

        let image = rest.starts_with("![");
        if image || rest.starts_with('[') {
            if let Some((label, consumed)) = link(&rest[usize::from(image)..]) {
                flush(&mut buf, &mut spans, style);
                if image {
                    spans.push(Span::styled(format!("[image: {label}]"), dim_style()));
                } else {
                    spans.push(Span::styled(
                        label.to_string(),
                        style.fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
                    ));
                }
                i += usize::from(image) + consumed;
                continue;
            }
        }

        if let Some(delim) = ["**", "__", "~~"].into_iter().find(|d| rest.starts_with(d)) {
            let open = match delim {
                "~~" => strike,
                _ => bold,
            };
            if open || rest[2..].contains(delim) {
                flush(&mut buf, &mut spans, style);
                if delim == "~~" {
                    strike = !strike;
                } else {
                    bold = !bold;
                }
            } else {
                buf.push_str(delim);
            }
            i += 2;
            continue;
        }

        let c = rest.chars().next().unwrap_or_default();
        if (c == '*' || c == '_') && emphasis_boundary(text, i, c, italic) {
            flush(&mut buf, &mut spans, style);
            italic = !italic;
            i += 1;
            continue;
        }

        buf.push(c);
        i += c.len_utf8();
    }

    flush(&mut buf, &mut spans, current(bold, italic, strike));
    spans
}

/// Parse `[label](target)` at the start of `rest`, returning the label and bytes consumed.
fn link(rest: &str) -> Option<(&str, usize)> {
    let close = rest.find("](")?;
    let label = &rest[1..close];
    if label.contains(']') {
        return None;
    }
    let target_end = rest[close + 2..].find(')')?;
    Some((label, close + 2 + target_end + 1))
}

/// Single `*` or `_` toggles italics only when it looks like emphasis, so
/// identifiers such as `snake_case_name` stay intact.
fn emphasis_boundary(text: &str, i: usize, marker: char, open: bool) -> bool {
    let before = text[..i].chars().next_back();
    let after = text[i + 1..].chars().next();
    if marker == '_'
        && before.is_some_and(char::is_alphanumeric)
        && after.is_some_and(char::is_alphanumeric)
    {
        return false;
    }
    if open {
        before.is_some_and(|c| !c.is_whitespace())
    } else {
        after.is_some_and(|c| !c.is_whitespace()) && text[i + 1..].contains(marker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_all(lines: &[String], width: u16) -> Vec<(usize, Line<'static>)> {
        render(lines, width, 0, usize::MAX)
    }

    fn lines(src: &str) -> Vec<String> {
        src.lines().map(String::from).collect()
    }

    fn text_of(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn span_with<'a>(line: &'a Line, content: &str) -> &'a Span<'a> {
        line.spans
            .iter()
            .find(|s| s.content == content)
            .unwrap_or_else(|| panic!("no span {content:?} in {line:?}"))
    }

    #[test]
    fn headings_drop_hash_marks_and_are_bold() {
        let out = render_all(&lines("# Title\n### Sub ###"), 80);
        assert_eq!(text_of(&out[0].1), "Title");
        assert_eq!(text_of(&out[1].1), "Sub");
        assert!(span_with(&out[0].1, "Title")
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn hash_without_space_is_not_a_heading() {
        let out = render_all(&lines("#hashtag"), 80);
        assert_eq!(text_of(&out[0].1), "#hashtag");
    }

    #[test]
    fn list_items_use_bullets_and_keep_numbers_and_tasks() {
        let out = render_all(
            &lines("- one\n  * two\n3. three\n- [x] done\n- [ ] todo"),
            80,
        );
        let texts: Vec<String> = out.iter().map(|(_, l)| text_of(l)).collect();
        assert_eq!(texts, ["• one", "  • two", "3. three", "☑ done", "☐ todo"]);
    }

    #[test]
    fn code_fences_are_hidden_and_their_body_is_not_parsed() {
        let out = render_all(&lines("```rust\n# not heading **x**\n```\nafter"), 80);
        let texts: Vec<String> = out.iter().map(|(_, l)| text_of(l)).collect();
        assert_eq!(texts, ["  # not heading **x**", "after"]);
        assert_eq!(out.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [1, 3]);
    }

    #[test]
    fn inline_markup_is_removed_and_styled() {
        let out = render_all(
            &lines("a **b** *c* `d` ~~e~~ [f](http://x) ![g](i.png)"),
            80,
        );
        let line = &out[0].1;
        assert_eq!(text_of(line), "a b c d e f [image: g]");
        assert!(span_with(line, "b")
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert!(span_with(line, "c")
            .style
            .add_modifier
            .contains(Modifier::ITALIC));
        assert_eq!(span_with(line, "d").style.fg, Some(Color::Yellow));
        assert!(span_with(line, "e")
            .style
            .add_modifier
            .contains(Modifier::CROSSED_OUT));
        assert!(span_with(line, "f")
            .style
            .add_modifier
            .contains(Modifier::UNDERLINED));
    }

    #[test]
    fn identifiers_and_lone_markers_stay_literal() {
        let out = render_all(&lines("call snake_case_name with 2 * 3 and a**b"), 80);
        assert_eq!(
            text_of(&out[0].1),
            "call snake_case_name with 2 * 3 and a**b"
        );
    }

    #[test]
    fn escaped_markers_are_shown_literally() {
        let out = render_all(&lines(r"\*not italic\* and \`tick\`"), 80);
        assert_eq!(text_of(&out[0].1), "*not italic* and `tick`");
    }

    #[test]
    fn rules_quotes_and_table_separators_render() {
        let out = render_all(&lines("---\n> quoted\n| a | b |\n|---|:-:|"), 12);
        assert_eq!(text_of(&out[0].1), "─".repeat(10));
        assert_eq!(text_of(&out[1].1), "│ quoted");
        assert_eq!(text_of(&out[2].1), "| a | b |");
        assert_eq!(out[3].1.spans[0].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn only_the_requested_window_is_rendered_and_fences_carry_over() {
        let src = lines("```\none\ntwo\n```\n# Head\nlast");
        let out = render(&src, 80, 2, 2);
        let got: Vec<(usize, String)> = out.iter().map(|(i, l)| (*i, text_of(l))).collect();
        assert_eq!(got, [(2, "  two".to_string()), (4, "Head".to_string())]);
    }

    #[test]
    fn very_long_lines_are_shown_as_written() {
        let line = format!("**{}**", "[".repeat(MAX_INLINE_BYTES));
        let out = render_all(std::slice::from_ref(&line), 80);
        assert_eq!(text_of(&out[0].1), line);
    }

    #[test]
    fn markdown_extensions_are_detected() {
        assert!(is_markdown_file(Path::new("README.md")));
        assert!(is_markdown_file(Path::new("notes.MARKDOWN")));
        assert!(!is_markdown_file(Path::new("main.rs")));
        assert!(!is_markdown_file(Path::new("page.mdx")));
    }
}
