//! MCP tool registry for automatic tool registration
//!
//! Provides a centralized registry for all MCP tools with their schemas.

use serde_json::{json, Value};

/// Tool definition for MCP protocol
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Tool name (must be unique)
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
    /// JSON Schema for input parameters
    pub input_schema: Value,
    /// Category for organization
    pub category: ToolCategory,
}

/// Tool categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    /// File operations (read, write, delete)
    File,
    /// Git operations (status, diff, commit)
    Git,
    /// Code analysis (symbols, definitions, references)
    Analysis,
    /// Dependency management (graph, imports)
    Dependency,
    /// Context optimization (smart context, tokens)
    Context,
    /// Project management (build, test, lint)
    Project,
}

impl ToolCategory {
    /// Get display name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::File => "File Operations",
            Self::Git => "Git Operations",
            Self::Analysis => "Code Analysis",
            Self::Dependency => "Dependency Analysis",
            Self::Context => "Context Optimization",
            Self::Project => "Project Management",
        }
    }
}

/// Get all registered tools
pub fn get_all_tools() -> Vec<ToolDefinition> {
    let mut tools = Vec::new();

    // File operations
    tools.extend(file_tools());

    // Git operations
    tools.extend(git_tools());

    // Analysis tools
    tools.extend(analysis_tools());

    // Dependency tools
    tools.extend(dependency_tools());

    // Context tools
    tools.extend(context_tools());

    // Project tools
    tools.extend(project_tools());

    tools
}

/// Get tools by category
pub fn get_tools_by_category(category: ToolCategory) -> Vec<ToolDefinition> {
    get_all_tools()
        .into_iter()
        .filter(|t| t.category == category)
        .collect()
}

/// File operation tools
fn file_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_directory",
            description: "List files and directories in a path",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path from root (optional, defaults to root)"
                    }
                }
            }),
            category: ToolCategory::File,
        },
        ToolDefinition {
            name: "get_tree",
            description: "Get directory tree structure",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path from root (optional, defaults to root)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum depth to traverse (optional, defaults to unlimited)"
                    }
                }
            }),
            category: ToolCategory::File,
        },
        ToolDefinition {
            name: "read_file",
            description: "Read content of a file",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    }
                },
                "required": ["path"]
            }),
            category: ToolCategory::File,
        },
        ToolDefinition {
            name: "read_files",
            description: "Read multiple files at once",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of relative paths to read"
                    }
                },
                "required": ["paths"]
            }),
            category: ToolCategory::File,
        },
        ToolDefinition {
            name: "write_file",
            description: "Write content to a file (create or overwrite)",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    },
                    "create_dirs": {
                        "type": "boolean",
                        "description": "Create parent directories if they don't exist. Default: false"
                    }
                },
                "required": ["path", "content"]
            }),
            category: ToolCategory::File,
        },
        ToolDefinition {
            name: "delete_file",
            description: "Delete a file or directory (moves to trash by default for safety)",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file or directory"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Delete directories recursively. Default: false"
                    },
                    "use_trash": {
                        "type": "boolean",
                        "description": "Move to trash instead of permanent deletion. Default: true"
                    }
                },
                "required": ["path"]
            }),
            category: ToolCategory::File,
        },
        ToolDefinition {
            name: "search_code",
            description: "Search for code patterns in the repository using grep/ripgrep",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The search pattern (regex supported)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional relative path to limit search scope"
                    }
                },
                "required": ["pattern"]
            }),
            category: ToolCategory::File,
        },
    ]
}

/// Git operation tools
fn git_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_git_status",
            description: "Get git status showing changed and staged files",
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            category: ToolCategory::Git,
        },
        ToolDefinition {
            name: "get_git_diff",
            description: "Get git diff for a specific file",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "staged": {
                        "type": "boolean",
                        "description": "If true, show staged changes (--cached). Default: false"
                    }
                },
                "required": ["path"]
            }),
            category: ToolCategory::Git,
        },
        ToolDefinition {
            name: "git_log",
            description: "Get git commit history",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of commits to return. Default: 10"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional path to filter commits"
                    }
                }
            }),
            category: ToolCategory::Git,
        },
        ToolDefinition {
            name: "stage_files",
            description: "Stage files for git commit",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of paths to stage. Empty or omit to stage all changes."
                    }
                }
            }),
            category: ToolCategory::Git,
        },
        ToolDefinition {
            name: "create_commit",
            description: "Create a git commit with staged changes",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Commit message"
                    }
                },
                "required": ["message"]
            }),
            category: ToolCategory::Git,
        },
    ]
}

/// Code analysis tools
fn analysis_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_file_symbols",
            description: "Extract code symbols (functions, classes, etc.) from a file",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    }
                },
                "required": ["path"]
            }),
            category: ToolCategory::Analysis,
        },
        ToolDefinition {
            name: "get_definitions",
            description: "Get function and class definitions from a file",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "line": {
                        "type": "integer",
                        "description": "Line number to get definition at (optional)"
                    },
                    "column": {
                        "type": "integer",
                        "description": "Column number (optional, requires line)"
                    }
                },
                "required": ["path"]
            }),
            category: ToolCategory::Analysis,
        },
        ToolDefinition {
            name: "get_references",
            description: "Find all references to a symbol",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "symbol": {
                        "type": "string",
                        "description": "Symbol name to find references for"
                    }
                },
                "required": ["path", "symbol"]
            }),
            category: ToolCategory::Analysis,
        },
        ToolDefinition {
            name: "get_diagnostics",
            description: "Get errors and warnings for a file (requires language-specific tools)",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    }
                },
                "required": ["path"]
            }),
            category: ToolCategory::Analysis,
        },
    ]
}

/// Dependency analysis tools
fn dependency_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_dependency_graph",
            description: "Get file dependency graph for a path",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path (file or directory)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum depth for transitive dependencies. Default: 2"
                    }
                },
                "required": ["path"]
            }),
            category: ToolCategory::Dependency,
        },
        ToolDefinition {
            name: "get_import_tree",
            description: "Get import/require tree for a file",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    }
                },
                "required": ["path"]
            }),
            category: ToolCategory::Dependency,
        },
        ToolDefinition {
            name: "find_circular_deps",
            description: "Find circular dependencies in the codebase",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to scan (optional, defaults to root)"
                    }
                }
            }),
            category: ToolCategory::Dependency,
        },
    ]
}

/// Context optimization tools
fn context_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_smart_context",
            description:
                "Get AI-optimized context for a file including dependencies and related tests",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "focus_file": {
                        "type": "string",
                        "description": "Main file to get context for"
                    },
                    "max_tokens": {
                        "type": "integer",
                        "description": "Maximum tokens to include. Default: 4000"
                    },
                    "include_deps": {
                        "type": "boolean",
                        "description": "Include dependency content. Default: true"
                    },
                    "include_tests": {
                        "type": "boolean",
                        "description": "Include related test files. Default: true"
                    }
                },
                "required": ["focus_file"]
            }),
            category: ToolCategory::Context,
        },
        ToolDefinition {
            name: "estimate_tokens",
            description: "Estimate token count for files",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of relative paths to estimate"
                    }
                },
                "required": ["paths"]
            }),
            category: ToolCategory::Context,
        },
        ToolDefinition {
            name: "compress_context",
            description: "Compress file content by removing comments and extra whitespace",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "max_tokens": {
                        "type": "integer",
                        "description": "Maximum tokens after compression (optional)"
                    }
                },
                "required": ["path"]
            }),
            category: ToolCategory::Context,
        },
    ]
}

/// Project management tools
fn project_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "run_build",
            description: "Run project build command",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Custom build command (optional, auto-detects by default)"
                    }
                }
            }),
            category: ToolCategory::Project,
        },
        ToolDefinition {
            name: "run_test",
            description: "Run project tests",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Specific test file or directory (optional)"
                    },
                    "filter": {
                        "type": "string",
                        "description": "Test name filter pattern (optional)"
                    }
                }
            }),
            category: ToolCategory::Project,
        },
        ToolDefinition {
            name: "run_lint",
            description: "Run linter on the project",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Specific path to lint (optional)"
                    },
                    "fix": {
                        "type": "boolean",
                        "description": "Attempt to auto-fix issues. Default: false"
                    }
                }
            }),
            category: ToolCategory::Project,
        },
        ToolDefinition {
            name: "get_project_stats",
            description: "Get project statistics (file counts, lines of code, etc.)",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to analyze (optional, defaults to root)"
                    }
                }
            }),
            category: ToolCategory::Project,
        },
    ]
}

/// Convert tool definitions to MCP Tool format
pub fn to_mcp_tools() -> Vec<super::types::Tool> {
    get_all_tools()
        .into_iter()
        .map(|t| super::types::Tool {
            name: t.name.to_string(),
            description: t.description.to_string(),
            input_schema: t.input_schema,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_tools() {
        let tools = get_all_tools();
        assert!(!tools.is_empty());

        // Check for some expected tools
        let tool_names: Vec<_> = tools.iter().map(|t| t.name).collect();
        assert!(tool_names.contains(&"read_file"));
        assert!(tool_names.contains(&"get_git_status"));
        assert!(tool_names.contains(&"get_smart_context"));
    }

    #[test]
    fn test_get_tools_by_category() {
        let file_tools = get_tools_by_category(ToolCategory::File);
        assert!(!file_tools.is_empty());
        for tool in &file_tools {
            assert_eq!(tool.category, ToolCategory::File);
        }
    }

    #[test]
    fn test_tool_schemas_valid() {
        let tools = get_all_tools();
        for tool in tools {
            // Verify schema is a valid object
            assert!(tool.input_schema.is_object());
            // Verify it has a type field
            assert!(tool.input_schema.get("type").is_some());
        }
    }

    #[test]
    fn test_unique_tool_names() {
        let tools = get_all_tools();
        let names: Vec<_> = tools.iter().map(|t| t.name).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len(), "Tool names must be unique");
    }
}
