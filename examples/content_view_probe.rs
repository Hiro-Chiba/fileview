//! Isolated content-search to bounded-view rendering experiment, not a product UI.

use std::path::Path;
use std::time::Instant;

use anyhow::{ensure, Context, Result};
use ratatui::{backend::TestBackend, widgets::Paragraph, Terminal};
use serde_json::json;

#[path = "support/content_probe.rs"]
mod content_probe;
#[path = "support/viewport_probe.rs"]
pub mod viewport_probe;

fn render_hit(hit: &content_probe::Hit, needle: &str) -> Result<(String, usize, bool)> {
    let anchor = viewport_probe::ViewportAnchor {
        line_number: hit.line_number,
        byte_offset: hit.line_start,
        stamp: hit.stamp.clone(),
    };
    let page = viewport_probe::read_match_viewport(
        &hit.path,
        &anchor,
        hit.match_offset,
        40,
        65536,
        &|| false,
    )?;
    let match_start = usize::try_from(
        hit.match_offset
            .checked_sub(page.byte_offset)
            .context("match precedes viewport")?,
    )?;
    let matched_text = page
        .text
        .get(match_start..)
        .context("match offset is outside the UTF-8 viewport")?;
    ensure!(
        matched_text.starts_with(needle),
        "hit offset does not identify matched text"
    );
    ensure!(page.first_line == hit.line_number, "wrong line number");
    // Align the matched column even when its byte offset fits within the read
    // budget but is beyond the terminal's visible width.
    let prefix_clipped = page.prefix_clipped || match_start > 0;
    let mut terminal = Terminal::new(TestBackend::new(120, 42))?;
    let text = matched_text
        .lines()
        .enumerate()
        .map(|(index, line)| {
            format!(
                "{:>9} {}{line}",
                page.first_line + index as u64,
                if index == 0 && prefix_clipped {
                    "… "
                } else {
                    ""
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    terminal.draw(|frame| frame.render_widget(Paragraph::new(text), frame.area()))?;
    let buffer = terminal.backend().buffer();
    let mut first_row = String::new();
    let mut x = 0;
    while x < 120 {
        let symbol = buffer[(x, 0)].symbol();
        first_row.push_str(symbol);
        x += ratatui::text::Span::raw(symbol).width().max(1) as u16;
    }
    // Reserve room for the line number and clipping marker. Wide queries only
    // need their first visible portion on one frame; the full match was checked above.
    let mut visible_prefix = String::new();
    for character in needle.chars() {
        let mut candidate = visible_prefix.clone();
        candidate.push(character);
        if ratatui::text::Span::raw(candidate.as_str()).width() > 100 {
            break;
        }
        visible_prefix = candidate;
    }
    ensure!(
        !visible_prefix.is_empty() && first_row.contains(&visible_prefix),
        "match prefix is not visible in the rendered frame"
    );
    Ok((
        first_row.trim_end().to_owned(),
        page.bytes_read,
        prefix_clipped,
    ))
}

fn run(path: &Path, needle: &str) -> Result<serde_json::Value> {
    let start = Instant::now();
    let mut first = None;
    let mut failure = None;
    let stats = content_probe::search(
        path,
        needle,
        &content_probe::Options::default(),
        &|| false,
        &mut |hit| {
            if first.is_none() {
                match render_hit(hit, needle) {
                    Ok((row, bytes, clipped)) => {
                        first = Some(json!({
                            "search_to_render_ms": start.elapsed().as_secs_f64() * 1000.0,
                            "line":hit.line_number,"line_start":hit.line_start,"match_offset":hit.match_offset,
                            "viewport_bytes_read":bytes,"prefix_clipped":clipped,"first_rendered_row":row,
                        }))
                    }
                    Err(error) => {
                        failure = Some(error);
                        return false;
                    }
                }
            }
            true
        },
    )?;
    if let Some(error) = failure {
        return Err(error);
    }
    ensure!(first.is_some(), "no matching line in fixture");
    Ok(
        json!({"first_result":first,"scan":stats,"scope":"streamed callback to offscreen ratatui frame; not full FileView TUI latency"}),
    )
}

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .context("usage: content_view_probe PATH LITERAL")?;
    let needle = args
        .next()
        .context("missing literal")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("literal must be UTF-8"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&run(Path::new(&path), &needle)?)?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streamed_search_renders_unicode_crlf_match_at_the_right_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file.txt");
        std::fs::write(&path, "first\r\nsecond\r\n日本語TARGET\r\nlast").unwrap();
        let result = run(&path, "日本語TARGET").unwrap();
        assert_eq!(result["first_result"]["line"], 3);
        assert_eq!(result["first_result"]["line_start"], 15);
    }

    #[test]
    fn far_match_on_a_giant_line_is_visible_without_loading_the_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("long.txt");
        std::fs::write(&path, format!("{}TARGET日本語", "あ".repeat(400_000))).unwrap();
        let result = run(&path, "TARGET日本語").unwrap();
        assert_eq!(result["first_result"]["line"], 1);
        assert_eq!(result["first_result"]["prefix_clipped"], true);
        assert!(
            result["first_result"]["viewport_bytes_read"]
                .as_u64()
                .unwrap()
                <= 65536
        );
    }
    #[test]
    fn ordinary_long_prefix_is_scrolled_to_a_japanese_match() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("long-prefix.txt");
        std::fs::write(&path, format!("{}日本語TARGET\n", "x".repeat(200))).unwrap();
        let result = run(&path, "日本語TARGET").unwrap();
        let row = result["first_result"]["first_rendered_row"]
            .as_str()
            .unwrap();
        assert!(row.contains("… 日本語TARGET"));
        assert_eq!(result["first_result"]["prefix_clipped"], true);
        assert_eq!(result["first_result"]["match_offset"], 200);
    }

    #[test]
    fn query_wider_than_terminal_renders_its_visible_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("wide-query.txt");
        let needle = "日本語".repeat(40);
        std::fs::write(&path, format!("prefix {needle}\n")).unwrap();
        let result = run(&path, &needle).unwrap();
        let row = result["first_result"]["first_rendered_row"]
            .as_str()
            .unwrap();
        assert!(row.contains(&"日本語".repeat(16)));
        assert!(!row.contains(&needle));
    }
}
