//! Structured predicates combined with nucleo fuzzy expressions.

use anyhow::{bail, Result};

use crate::git::FileStatus;

/// Validated workspace search expression.
#[derive(Debug, Default)]
pub struct WorkspaceQuery {
    pub(crate) text: String,
    extension: Option<String>,
    directory: Option<bool>,
    git: Option<String>,
}

impl WorkspaceQuery {
    /// Parse filters. Repeated predicates and invalid values are rejected.
    pub fn parse(input: &str) -> Result<Self> {
        if input.len() > 4096 {
            bail!("workspace query exceeds 4096 bytes");
        }
        let mut query = Self::default();
        let mut words = Vec::new();
        for word in input.split_whitespace() {
            if let Some(value) = word.strip_prefix("ext:") {
                if value.is_empty() || value.contains(['/', '\\']) || query.extension.is_some() {
                    bail!("expected one nonempty ext: extension");
                }
                query.extension = Some(value.trim_start_matches('.').to_owned());
                if query.extension.as_deref() == Some("") {
                    bail!("extension must not be empty");
                }
            } else if let Some(value) = word.strip_prefix("type:") {
                if query.directory.is_some() {
                    bail!("type: may only appear once");
                }
                query.directory = Some(match value {
                    "file" => false,
                    "dir" => true,
                    _ => bail!("type: must be file or dir"),
                });
            } else if let Some(value) = word.strip_prefix("git:") {
                if query.git.is_some() || !matches!(value, "modified" | "changed" | "untracked") {
                    bail!("expected one git:modified, git:changed, or git:untracked filter");
                }
                query.git = Some(value.to_owned());
            } else {
                words.push(word);
            }
        }
        query.text = words.join(" ");
        Ok(query)
    }

    pub(crate) fn matches(&self, entry: &super::index::Entry, status: FileStatus) -> bool {
        self.matches_kind(entry.is_dir)
            && self
                .extension
                .as_ref()
                .is_none_or(|value| entry.extension.eq_ignore_ascii_case(value))
            && self.git.as_deref().is_none_or(|value| match value {
                "modified" => status == FileStatus::Modified,
                "untracked" => status == FileStatus::Untracked,
                _ => !matches!(status, FileStatus::Clean | FileStatus::Ignored),
            })
    }

    pub(crate) fn matches_kind(&self, is_dir: bool) -> bool {
        self.directory.is_none_or(|value| value == is_dir) && (self.extension.is_none() || !is_dir)
    }

    pub(crate) fn needs_git(&self) -> bool {
        self.git.is_some()
    }
}
