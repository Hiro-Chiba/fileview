# Claude Code Integration Guide

FileView is designed to work seamlessly with Claude Code and other AI coding assistants.

## Quick Start

```bash
# Install
cargo install fileview

# Basic usage with Claude Code
fv --tree --depth 2 ./src          # Show context
selected=$(fv --select-mode)       # Pick files
fv --mcp-server                    # Run as MCP server
```

## Features

### 1. Tree Output (`--tree`)

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

#### Available Tools

| Tool | Description |
|------|-------------|
| `list_directory` | List files in a directory |
| `get_tree` | Get directory tree structure |
| `read_file` | Read file contents |

#### Example Requests

```json
// List directory
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_directory","arguments":{"path":"src"}}}

// Get tree
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_tree","arguments":{"path":"src","depth":2}}}

// Read file
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_file","arguments":{"path":"src/main.rs"}}}
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
| `--tree` | Output directory tree to stdout |
| `--depth N` | Limit tree depth (default: unlimited) |
| `--with-content` | Include file contents in tree output |
| `--select-mode` | Simple selection mode (Enter to confirm) |
| `--multi` | Allow multiple selection |
| `--mcp-server` | Run as MCP server |
| `--pick` | Pick mode (same as `--select-mode`) |
| `--format FMT` | Output format: lines, null, json |

## Keybindings

| Key | Action |
|-----|--------|
| `Y` | Copy file content to clipboard |
| `Ctrl+Y` | Copy in Claude-friendly format |
| `Enter` | Select (in select mode) |
| `Space` | Toggle mark (for multi-select) |

## Tips

1. **Context Window**: Use `--depth` to limit tree depth for large projects
2. **Multiple Files**: Use `Ctrl+Y` to copy multiple files at once
3. **MCP Root**: Pass project path to `--mcp-server` to set the root directory
4. **Narrow Terminals**: FileView adapts to narrow terminals (80x24) in Claude Code

## Comparison with Other Tools

| Feature | fileview | yazi | lf | ranger |
|---------|:--------:|:----:|:--:|:------:|
| MCP Server | Yes | No | No | No |
| Tree output | Yes | No | No | No |
| Claude format | Yes | No | No | No |
| Select mode | Yes | No | No | No |

FileView is the only terminal file manager with native AI tooling support.
