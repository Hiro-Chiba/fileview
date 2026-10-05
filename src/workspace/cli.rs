//! Non-interactive file search using the same engine as Ctrl+P and MCP.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use super::WorkspaceEngine;

/// Help text for file search.
pub const HELP: &str = "Usage: fv search QUERY [PATH] [--hidden] [--json] [--limit N]

Find files by name or relative path. PATH defaults to the current directory.
Use text:WORD to search file contents (literal, case-sensitive; requires rg).
Use Ctrl+P in the TUI for the same search.

Options:
    -a, --hidden  Include hidden paths (ignore rules still apply)
    --json        Output JSON for scripts and AI tools
    --limit N     Maximum results, from 1 to 1000 (default: 100)
    -h, --help    Show this help

Examples:
    fv search main
    fv search 'text:TODO' --json
    fv search 'ext:rs git:changed' ./src --json

Optional filters: ext:rs, type:file, type:dir, git:changed,
                  git:modified, git:untracked.
Quote queries containing spaces. Use -- before a query starting with a dash.";

/// Parse and execute arguments following `fv search`.
pub fn run(args: &[OsString]) -> Result<()> {
    let mut positional = Vec::new();
    let mut hidden = false;
    let mut json = false;
    let mut limit = 100usize;
    let mut options = true;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if !options {
            positional.push(arg);
            continue;
        }
        match arg.to_str() {
            Some("--") => options = false,
            Some("--help" | "-h") => {
                println!("{HELP}");
                return Ok(());
            }
            Some("--hidden" | "-a") => hidden = true,
            Some("--json") => json = true,
            Some("--limit") => {
                limit = iter
                    .next()
                    .and_then(|s| s.to_str())
                    .context("--limit requires an integer from 1 to 1000")?
                    .parse()
                    .context("--limit requires an integer from 1 to 1000")?;
                if !(1..=1000).contains(&limit) {
                    bail!("--limit must be between 1 and 1000");
                }
            }
            Some(s) if s.starts_with('-') => {
                bail!("Unknown search option: {s}. Use -- before a query starting with a dash.")
            }
            _ => positional.push(arg),
        }
    }
    if positional.is_empty() || positional.len() > 2 {
        bail!("Expected QUERY and an optional PATH.\n{HELP}");
    }
    let query = positional[0]
        .to_str()
        .context("Query must be valid UTF-8")?;
    let root = match positional.get(1) {
        Some(path) => PathBuf::from(path.as_os_str()),
        None => std::env::current_dir()?,
    };
    search(&root, query, hidden, json, limit)
}

fn search(
    root: &std::path::Path,
    query: &str,
    hidden: bool,
    json: bool,
    limit: usize,
) -> Result<()> {
    if let Some(literal) = query.strip_prefix("text:") {
        let mut hits = Vec::new();
        super::content::search(root, literal, hidden, limit, &|| false, &mut |hit| {
            hits.push(hit)
        })?;
        if json {
            let values: Vec<_> = hits.iter().map(|hit| {
                let path = hit.path.to_str().context("JSON output cannot represent a non-UTF8 path")?;
                Ok(serde_json::json!({"path":path,"line":hit.line_number,"byte_offset":hit.match_offset,"text":hit.snippet}))
            }).collect::<Result<_>>()?;
            println!("{}", serde_json::to_string(&values)?);
        } else {
            for hit in hits {
                println!("{}:{}:{}", hit.path.display(), hit.line_number, hit.snippet);
            }
        }
        return Ok(());
    }
    super::query::WorkspaceQuery::parse(query)?;
    let engine = WorkspaceEngine::new(root)?;
    engine.wait_ready(Duration::from_secs(30))?;
    let matches = engine.search(query, hidden, limit)?;
    if json {
        let paths: Vec<_> = matches.iter().map(|m| {
            let path = m.path.to_str().context("JSON output cannot represent a non-UTF8 path")?;
            Ok(serde_json::json!({"path": path, "display": m.display, "score": m.score, "is_dir": m.is_dir, "size": m.size}))
        }).collect::<Result<_>>()?;
        println!("{}", serde_json::to_string(&paths)?);
    } else {
        for item in matches {
            println!("{}", item.display);
        }
    }
    Ok(())
}
