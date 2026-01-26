//! Tree entry (node) definition

use std::path::PathBuf;

/// A single entry in the file tree
#[derive(Debug, Clone)]
pub struct TreeEntry {
    /// Full path to the entry
    pub path: PathBuf,
    /// Display name
    pub name: String,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Depth in the tree (0 = root)
    pub depth: usize,
    /// Whether directory is expanded
    pub expanded: bool,
    /// Child entries (directories only)
    children: Vec<TreeEntry>,
}

impl TreeEntry {
    /// Create a new tree entry
    pub fn new(path: PathBuf, depth: usize) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let is_dir = path.is_dir();

        Self {
            path,
            name,
            is_dir,
            depth,
            expanded: false,
            children: Vec::new(),
        }
    }

    /// Check if this entry is expanded
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Get children (immutable)
    pub fn children(&self) -> &[TreeEntry] {
        &self.children
    }

    /// Get children (mutable)
    pub fn children_mut(&mut self) -> &mut Vec<TreeEntry> {
        &mut self.children
    }

    /// Toggle expanded state
    pub fn toggle_expanded(&mut self) {
        if self.is_dir {
            self.expanded = !self.expanded;
        }
    }

    /// Set expanded state
    pub fn set_expanded(&mut self, expanded: bool) {
        if self.is_dir {
            self.expanded = expanded;
        }
    }

    /// Load children from filesystem
    pub fn load_children(&mut self, show_hidden: bool) -> anyhow::Result<()> {
        if !self.is_dir {
            return Ok(());
        }

        self.children.clear();
        let mut entries: Vec<_> = std::fs::read_dir(&self.path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                if show_hidden {
                    true
                } else {
                    !e.file_name().to_string_lossy().starts_with('.')
                }
            })
            .map(|e| TreeEntry::new(e.path(), self.depth + 1))
            .collect();

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        self.children = entries;
        Ok(())
    }
}
