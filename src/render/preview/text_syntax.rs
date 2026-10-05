//! Cancellable syntax replay for a bounded page. No whole-file text is retained.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{ensure, Result};
use ratatui::style::Color;
use syntect::easy::HighlightLines;

use super::{get_syntax_set, get_theme, StyledLine, StyledSegment};
use crate::render::preview::window::{FileStamp, TextWindow};

const MAX_LINE: usize = 64 * 1024;
const MAX_REPLAY: u64 = 8 * 1024 * 1024;
const REPLAY_TIME: Duration = Duration::from_millis(200);

pub(super) struct SyntaxResult {
    pub lines: Option<Vec<StyledLine>>,
    pub note: Option<&'static str>,
}
impl SyntaxResult {
    fn plain(note: Option<&'static str>) -> Self {
        Self { lines: None, note }
    }
}

enum InputLine {
    Line(Vec<u8>),
    End,
    Limit(&'static str),
}

fn read_line(
    input: &mut impl BufRead,
    consumed: &mut u64,
    started: Instant,
    cancelled: &dyn Fn() -> bool,
) -> Result<InputLine> {
    let mut line = Vec::new();
    loop {
        ensure!(!cancelled(), "preview cancelled");
        if started.elapsed() >= REPLAY_TIME {
            return Ok(InputLine::Limit("plain: syntax time limit"));
        }
        let bytes = input.fill_buf()?;
        if bytes.is_empty() {
            return Ok(if line.is_empty() {
                InputLine::End
            } else {
                InputLine::Line(line)
            });
        }
        let length = bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |n| n + 1);
        if *consumed + length as u64 > MAX_REPLAY {
            return Ok(InputLine::Limit("plain: syntax replay limit"));
        }
        if line.len() + length > MAX_LINE {
            return Ok(InputLine::Limit("plain: long line"));
        }
        let ended = bytes[length - 1] == b'\n';
        line.extend_from_slice(&bytes[..length]);
        *consumed += length as u64;
        input.consume(length);
        if ended {
            return Ok(InputLine::Line(line));
        }
    }
}

/// Replay full lines so multiline grammar state and end-of-line expressions are
/// identical to whole-file highlighting, then retain only visible UTF-8 slices.
pub(super) fn highlight(
    path: &Path,
    window: &TextWindow,
    cancelled: &dyn Fn() -> bool,
) -> Result<SyntaxResult> {
    ensure!(!cancelled(), "preview cancelled");
    if window.next_offset > MAX_REPLAY {
        return Ok(SyntaxResult::plain(Some("plain: syntax replay limit")));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    ensure!(
        FileStamp::from_metadata(&file.metadata()?)? == window.anchor.stamp,
        "file changed before syntax replay"
    );
    let mut input = BufReader::with_capacity(8192, file);
    let ss = get_syntax_set();
    let theme = get_theme();
    // Shared grammar initialization may wait for another worker. It is not
    // part of the per-request replay budget. Each regex call is cooperative
    // work and cannot itself be interrupted by the cancellation callback.
    let started = Instant::now();
    let mut consumed = 0u64;
    let first = read_line(&mut input, &mut consumed, started, cancelled)?;
    let mut bytes = match first {
        InputLine::Line(line) => line,
        InputLine::End => {
            return Ok(SyntaxResult {
                lines: Some(Vec::new()),
                note: None,
            })
        }
        InputLine::Limit(note) => return Ok(SyntaxResult::plain(Some(note))),
    };
    let Ok(first_text) = std::str::from_utf8(&bytes) else {
        return Ok(SyntaxResult::plain(Some("plain: non-UTF-8 syntax context")));
    };
    let Some(syntax) = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| ss.find_syntax_by_extension(ext))
        .or_else(|| ss.find_syntax_by_first_line(first_text.trim_end_matches(['\r', '\n'])))
    else {
        return Ok(SyntaxResult::plain(None));
    };
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut styled = Vec::new();
    let mut line_start = 0u64;
    loop {
        ensure!(!cancelled(), "preview cancelled");
        if started.elapsed() >= REPLAY_TIME {
            return Ok(SyntaxResult::plain(Some("plain: syntax time limit")));
        }
        let Ok(line) = std::str::from_utf8(&bytes) else {
            return Ok(SyntaxResult::plain(Some("plain: non-UTF-8 syntax context")));
        };
        let Ok(ranges) = highlighter.highlight_line(line, ss) else {
            return Ok(SyntaxResult::plain(Some("plain: syntax unavailable")));
        };
        let line_end = line_start + bytes.len() as u64;
        if line_end > window.byte_offset && line_start < window.next_offset {
            let clip_start = window.byte_offset.saturating_sub(line_start) as usize;
            let clip_end = (window.next_offset - line_start).min(bytes.len() as u64) as usize;
            let mut offset = 0usize;
            let mut segments = Vec::new();
            for (style, text) in ranges {
                let start = clip_start.max(offset);
                let end = clip_end.min(offset + text.len());
                if start < end {
                    segments.push(StyledSegment {
                        text: line[start..end].to_owned(),
                        color: Color::Rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        ),
                    });
                }
                offset += text.len();
            }
            styled.push(StyledLine { segments });
        }
        // Bound pathological scope nesting as well as retained text. The parser
        // has private internal state, so the replay byte limit also remains.
        let (highlight_state, parse_state) = highlighter.state();
        if highlight_state.path.len() > 1024 {
            return Ok(SyntaxResult::plain(Some("plain: syntax nesting limit")));
        }
        highlighter = HighlightLines::from_state(theme, highlight_state, parse_state);
        if line_end >= window.next_offset {
            break;
        }
        line_start = line_end;
        bytes = match read_line(&mut input, &mut consumed, started, cancelled)? {
            InputLine::Line(line) => line,
            InputLine::End => break,
            InputLine::Limit(note) => return Ok(SyntaxResult::plain(Some(note))),
        };
    }
    ensure!(!cancelled(), "preview cancelled");
    ensure!(
        FileStamp::from_metadata(&input.get_ref().metadata()?)? == window.anchor.stamp
            && FileStamp::capture(path)? == window.anchor.stamp,
        "file changed during syntax replay"
    );
    ensure!(
        styled.len() == window.text.lines().count(),
        "file changed during syntax replay"
    );
    Ok(SyntaxResult {
        lines: Some(styled),
        note: None,
    })
}
