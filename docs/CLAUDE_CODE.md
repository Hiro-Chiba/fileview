# Claude Code Integration Guide

FileView is designed to work seamlessly with Claude Code and other AI coding assistants.

## Quick Start

```bash
# Install
cargo install fileview

# Basic usage with Claude Code
fv --context                       # Project overview
fv --tree --depth 2 ./src          # Show tree
selected=$(fv --select-mode)       # Pick files
fv --mcp-server                    # Run as MCP server
```

## Features

### 1. Project Context (`--context`)

Output AI-friendly project overview:

```bash
fv --context
```

Output:
```markdown
## Project: myapp

**Branch:** main (clean)
**Recent:** feat: add user auth (2h ago)

### Structure:
├── src/
│   ├── app/
│   ├── handlers/
│   └── main.rs
└── tests/

### Stats: 45 Rust files, ~12k lines
Types: 40 Rust files, 3 TOML files, 2 Markdown files
```

### 2. Tree Output (`--tree`)

Output directory structure for AI context:

```bash
# Basic tree
fv --tree ./src

# Limit depth
fv --tree --depth 2 ./project

# Include file contents
fv --tree --with-content ./src
```

Output:
```
/src
├── main.rs
├── lib.rs
└── utils/
    ├── mod.rs
    └── helpers.rs
```

### 2. Select Mode (`--select-mode`)

Interactive file picker for AI workflows:

```bash
# Single selection
selected=$(fv --select-mode)
echo "Analyze: $selected"

# Multiple selection
selected=$(fv --select-mode --multi)
echo "Selected files: $selected"
```

### 3. Claude-Friendly Copy (`Ctrl+Y`)

Press `Ctrl+Y` in FileView to copy files in Claude-friendly format:

```markdown
### File: src/main.rs
```rs
fn main() {
    println!("Hello");
}
```

### File: src/lib.rs
```rs
pub mod utils;
```
```

This format includes:
- File paths as headers
- Syntax hints (```rs, ```py, etc.)
- Multiple files in one clipboard

### 4. MCP Server (`--mcp-server`)

Run FileView as an MCP (Model Context Protocol) server:

```bash
fv --mcp-server
```

#### Configuration

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "fileview": {
      "command": "fv",
      "args": ["--mcp-server"]
    }
  }
}
```

Or for Claude Code CLI (`~/.claude.json`):

```json
{
  "mcpServers": {
    "fileview": {
      "command": "fv",
      "args": ["--mcp-server", "/path/to/project"]
    }
  }
}
```

#### Available Tools (MCP 2.0 - 21 tools)

##### File Operations
| Tool | Description |
|------|-------------|
| `list_directory` | List files in a directory |
| `get_tree` | Get directory tree structure |
| `read_file` | Read single file contents |
| `read_files` | Read multiple files at once |
| `write_file` | Create or update file (with `create_dirs` option) |
| `delete_file` | Delete file/directory (trash support, recursive) |
| `search_code` | Search code with grep/ripgrep |

##### Git Operations
| Tool | Description |
|------|-------------|
| `get_git_status` | Get git changed/staged files |
| `get_git_diff` | Get file diff (staged/unstaged) |
| `git_log` | Get commit history with optional path filter |
| `stage_files` | Stage files for commit |
| `create_commit` | Create commit with message |

##### Code Analysis
| Tool | Description |
|------|-------------|
| `get_file_symbols` | Extract functions, classes, structs from file |
| `get_definitions` | Find symbol definitions (line/column) |
| `get_references` | Find all references to a symbol |
| `get_diagnostics` | Get errors and warnings |

##### Dependency Analysis
| Tool | Description |
|------|-------------|
| `get_dependency_graph` | Build dependency graph with petgraph |
| `get_import_tree` | Get import/require tree |
| `find_circular_deps` | Detect circular dependencies |

##### AI Context Optimization
| Tool | Description |
|------|-------------|
| `get_smart_context` | AI-optimized context with dependency awareness |
| `estimate_tokens` | Estimate token count (tiktoken-rs) |
| `compress_context` | Compress content for AI context |

##### Project Management
| Tool | Description |
|------|-------------|
| `run_build` | Run build command (auto-detect: cargo/npm/make) |
| `run_test` | Run tests with optional filter |
| `run_lint` | Run linter with optional auto-fix |
| `get_project_stats` | Get project statistics |

#### Example Requests

```json
// List directory
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_directory","arguments":{"path":"src"}}}

// Get tree
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_tree","arguments":{"path":"src","depth":2}}}

// Read multiple files
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_files","arguments":{"paths":["src/main.rs","src/lib.rs"]}}}

// Git status
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_git_status","arguments":{}}}

// Smart context (AI-optimized)
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_smart_context","arguments":{"focus_file":"src/main.rs","max_tokens":4000}}}

// Dependency graph
{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"get_dependency_graph","arguments":{"path":"src/"}}}

// Estimate tokens
{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"estimate_tokens","arguments":{"paths":["src/main.rs","src/lib.rs"]}}}

// Run tests
{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"run_test","arguments":{"filter":"test_"}}}
```

## Workflow Examples

### Code Review

```bash
# 1. Show project structure to Claude
fv --tree --depth 2 .

# 2. Select files for review
files=$(fv --select-mode --multi)

# 3. Copy content for Claude
# In fileview: navigate to files and press Ctrl+Y
```

### Bug Investigation

```bash
# 1. Show structure
fv --tree ./src

# 2. Let Claude browse via MCP
# Claude: "Let me check the error handling in src/handlers"
```

### Refactoring

```bash
# 1. Select files to refactor
files=$(fv --select-mode --multi)
echo "Refactoring: $files"

# 2. Use tree for context
fv --tree --with-content ./src/module
```

## CLI Reference

| Option | Description |
|--------|-------------|
| `--context` | Output project context (AI-friendly markdown) |
| `--context-pack P` | Output context pack preset (`minimal`, `review`, `debug`, `refactor`, `incident`, `onboarding`) |
| `--context-format F` | Context pack format: `ai-md`, `jsonl` |
| `--agent A` | Agent profile: `claude`, `codex`, `cursor` |
| `--token-budget N` | Token budget for context packs |
| `--include-git-diff` | Force include git diff summary in context packs |
| `--include-tests` | Include inferred tests in context packs |
| `--context-depth N` | Fallback scan depth for context packs |
| `--tree` | Output directory tree to stdout |
| `--depth N` | Limit tree depth (default: unlimited) |
| `--with-content` | Include file contents in tree output |
| `--select-mode` | Simple selection mode (Enter to confirm) |
| `--multi` | Allow multiple selection |
| `--select-related F` | Output related file paths for `F` |
| `--explain-selection` | Include score/reasons for `--select-related` output |
| `--mcp-server` | Run as MCP server |
| `--pick` | Pick mode (same as `--select-mode`) |
| `--format FMT` | Output format: lines, null, json |

## Keybindings

### Smart Selection

| Key | Action |
|-----|--------|
| `Ctrl+G` | Select all git changed files |
| `Ctrl+T` | Select test file pair (auto-detect) |
| `Ctrl+R` | Select related files (same stem/test pairs) |
| `Ctrl+E` | Select error-context files (logs/traces) |

Test pair patterns:
- `foo.rs` → `foo_test.rs`, `test_foo.rs`, `tests/foo.rs`
- `foo.ts` → `foo.test.ts`, `foo.spec.ts`
- `foo.py` → `test_foo.py`, `foo_test.py`

### Copy & Selection

| Key | Action |
|-----|--------|
| `Y` | Copy file content to clipboard |
| `Ctrl+Y` | Copy in Claude-friendly format |
| `Ctrl+Shift+Y` | Copy context pack (minimal preset) |
| `Ctrl+A` | Toggle AI focus mode (ultra-compact + peek preview) |
| `Ctrl+Shift+P` | Open AI history popup |
| `Enter` | Select (in select mode) |
| `Space` | Toggle mark (for multi-select) |

AI history entries now include metadata (`preset`, token estimate, file count) so you can quickly re-use the right context snapshot.

## Tips

1. **Context Window**: Use `--depth` to limit tree depth for large projects
2. **Multiple Files**: Use `Ctrl+Y` to copy multiple files at once
3. **MCP Root**: Pass project path to `--mcp-server` to set the root directory
4. **Narrow Terminals**: FileView adapts to terminals as narrow as 20 characters
5. **Token Optimization**: Use `estimate_tokens` to check context size before sending to AI
6. **Smart Context**: Use `get_smart_context` to get dependency-aware context
7. **Dependency Analysis**: Use `find_circular_deps` to detect circular dependencies

## Ultra-Narrow Terminal Support (v2.0)

FileView v2.0 supports terminals as narrow as **20 characters**:

| Width | Mode | Features |
|-------|------|----------|
| 80+ | Full | All features, icons, full preview |
| 40-79 | Compact | Condensed status, partial preview |
| 25-39 | Narrow | No icons, minimal status |
| 20-24 | Ultra | Essential info only |

This makes FileView perfect for AI pair programming where Claude Code uses 80% of the screen.

## Comparison with Other Tools

| Feature | fileview | yazi | lf | ranger |
|---------|:--------:|:----:|:--:|:------:|
| MCP Server | ✅ 21 tools | ❌ | ❌ | ❌ |
| Dependency analysis | ✅ | ❌ | ❌ | ❌ |
| Token estimation | ✅ | ❌ | ❌ | ❌ |
| Smart context | ✅ | ❌ | ❌ | ❌ |
| Code analysis | ✅ | ❌ | ❌ | ❌ |
| Git operations | ✅ | ❌ | ❌ | ❌ |
| Project management | ✅ | ❌ | ❌ | ❌ |
| 20-char terminal | ✅ | ❌ | △ | ❌ |
| Tree output | ✅ | ❌ | ❌ | ❌ |
| Claude format | ✅ | ❌ | ❌ | ❌ |

FileView is the only terminal file manager with native AI tooling support.
