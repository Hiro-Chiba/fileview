//! Read-only workspace search, retaining the index for the server lifetime.

use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::{error_result, success_result, ToolCallResult};
use crate::workspace::WorkspaceEngine;

/// Execute a workspace query using the server-owned engine.
pub fn search(root: &Path, args: &Value, engine: &mut Option<WorkspaceEngine>) -> ToolCallResult {
    match search_inner(root, args, engine) {
        Ok(value) => success_result(value.to_string()),
        Err(error) => error_result(&error.to_string()),
    }
}

fn search_inner(root: &Path, args: &Value, engine: &mut Option<WorkspaceEngine>) -> Result<Value> {
    let args = args.as_object().context("Arguments must be an object")?;
    for name in args.keys() {
        match name.as_str() {
            "query" | "show_hidden" | "limit" => {}
            "saved_query" => bail!("saved_query is no longer supported; pass query instead"),
            _ => bail!("Unknown search_workspace argument: {name}"),
        }
    }
    let query = args
        .get("query")
        .context("Missing required parameter: query")?
        .as_str()
        .context("query must be a string")?;
    let hidden = match args.get("show_hidden") {
        Some(value) => value.as_bool().context("show_hidden must be a boolean")?,
        None => false,
    };
    let limit = match args.get("limit") {
        Some(value) => value
            .as_u64()
            .filter(|n| (1..=1000).contains(n))
            .context("limit must be between 1 and 1000")? as usize,
        None => 100,
    };
    if let Some(literal) = query.strip_prefix("text:") {
        let root = root.canonicalize()?;
        let mut hits = Vec::new();
        let summary = crate::workspace::content::search(
            &root,
            literal,
            hidden,
            limit,
            &|| false,
            &mut |hit| hits.push(hit),
        )?;
        let matches = hits.iter().map(|hit| {
            let path = hit.path.strip_prefix(&root)?.to_str().context("MCP cannot represent a non-UTF8 path")?;
            Ok(json!({"path":path,"line":hit.line_number,"byte_offset":hit.match_offset,"text":hit.snippet}))
        }).collect::<Result<Vec<_>>>()?;
        return Ok(
            json!({"matches":matches,"limit_reached":summary.limit_reached,"changed_results":summary.changed_results}),
        );
    }
    // Validate before starting an index for malformed tool calls.
    crate::workspace::query::WorkspaceQuery::parse(query)?;
    let root = root.canonicalize()?;
    if engine.as_ref().is_none_or(|engine| engine.root() != root) {
        *engine = Some(WorkspaceEngine::new(&root)?);
    }
    let engine = engine.as_ref().context("Workspace engine unavailable")?;
    engine.wait_ready(Duration::from_secs(30))?;
    let matches = engine.search(query, hidden, limit)?;
    let paths: Vec<_> = matches
        .iter()
        .map(|m| {
            let path = m
                .path
                .strip_prefix(&root)?
                .to_str()
                .context("MCP cannot represent a non-UTF8 path")?;
            Ok(json!({
                "path": path, "score": m.score, "is_dir": m.is_dir, "size": m.size,
            }))
        })
        .collect::<Result<_>>()?;
    let status = engine.status();
    Ok(
        json!({"matches": paths, "indexed_entries": status.entries, "refreshing": status.refreshing, "revision": status.revision, "watching": status.watching, "error": status.error}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_search_has_same_literal_semantics_and_needs_no_index() {
        if std::process::Command::new("rg")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("sample.rs"), "first\n日本語 TODO.*\n").unwrap();
        let mut engine = None;
        let result =
            search_inner(root.path(), &json!({"query":"text:TODO.*"}), &mut engine).unwrap();
        assert!(engine.is_none());
        assert_eq!(result["matches"][0]["path"], "sample.rs");
        assert_eq!(result["matches"][0]["line"], 2);
        assert_eq!(result["matches"][0]["byte_offset"], 16);
    }

    #[test]
    fn rejects_invalid_arguments_without_starting_engine() {
        let root = tempfile::tempdir().unwrap();
        let mut engine = None;
        for args in [
            json!({}),
            json!(null),
            json!([]),
            json!({"query": null}),
            json!({"query": 42}),
            json!({"query": "", "unknown": true}),
            json!({"query": "", "root": "/elsewhere"}),
            json!({"query": "", "limit": 1001}),
            json!({"query": "", "show_hidden": "true"}),
        ] {
            assert_eq!(search(root.path(), &args, &mut engine).is_error, Some(true));
            assert!(engine.is_none());
        }
    }

    #[test]
    fn rejects_obsolete_saved_queries_without_starting_engine() {
        let root = tempfile::tempdir().unwrap();
        let mut engine = None;
        for args in [
            json!({"saved_query": "rust"}),
            json!({"query": "ext:rs", "saved_query": "rust"}),
        ] {
            let result = search(root.path(), &args, &mut engine);
            assert_eq!(result.is_error, Some(true));
            assert!(result.content[0].text.contains("pass query instead"));
            assert!(engine.is_none());
        }
    }

    #[test]
    fn search_reuses_index_and_obeys_hidden_and_root_boundaries() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("visible.rs"), "").unwrap();
        std::fs::write(root.path().join(".hidden.rs"), "").unwrap();
        std::fs::write(other.path().join("other.rs"), "").unwrap();
        let mut engine = None;
        let result = search_inner(root.path(), &json!({"query":"ext:rs"}), &mut engine).unwrap();
        assert_eq!(result["matches"].as_array().unwrap().len(), 1);
        let result = search_inner(
            root.path(),
            &json!({"query":"ext:rs", "show_hidden":true}),
            &mut engine,
        )
        .unwrap();
        assert_eq!(result["matches"].as_array().unwrap().len(), 2);
        let result = search_inner(other.path(), &json!({"query":"ext:rs"}), &mut engine).unwrap();
        assert_eq!(result["matches"][0]["path"], "other.rs");
    }
    #[cfg(unix)]
    #[test]
    fn rejects_unrepresentable_paths_without_lossy_aliases() {
        use std::os::unix::ffi::OsStringExt;
        let root = tempfile::tempdir().unwrap();
        let name = std::ffi::OsString::from_vec(vec![b'a', 0xff]);
        if std::fs::write(root.path().join(name), "").is_err() {
            return;
        }
        let mut engine = Some(WorkspaceEngine::with_cache(root.path(), None).unwrap());
        let result = search(root.path(), &json!({"query":""}), &mut engine);
        assert_eq!(result.is_error, Some(true));
        assert!(result.content[0].text.contains("non-UTF8"));
    }
}
