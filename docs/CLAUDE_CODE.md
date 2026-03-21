# Claude Code Integration Guide

FileView can be used with Claude Code as a file browser and MCP server.

## Quick Start

```bash
# Project context
fv --context

# Tree output
fv --tree --depth 2 ./src

# File selection
selected=$(fv --select-mode --multi)

# Run as MCP server
fv --mcp-server
```

## CLI Options (AI-related)

| Option | Description |
|--------|-------------|
| `--context` | Output project context as AI-friendly markdown |
| `--context-pack P` | Context pack preset (`minimal`, `review`, `debug`, `refactor`, `incident`, `onboarding`) |
| `--context-format F` | Format: `ai-md`, `jsonl` |
| `--agent A` | Agent profile: `claude`, `codex`, `cursor` |
| `--token-budget N` | Token budget for context packs |
| `--include-git-diff` | Include git diff summary |
| `--include-tests` | Include inferred test files |
| `--context-depth N` | Fallback scan depth |
| `--select-related F` | Output related file paths |
| `--explain-selection` | Include score/reasons for `--select-related` |
| `--resume-ai-session [NAME]` | Restore AI session metadata (default: `ai`) |
| `benchmark ai` | Run AI workflow benchmarks |
| `init claude` | Add fileview MCP entry to Claude config |

## Keybindings (AI-related)

| Key | Action |
|-----|--------|
| `Ctrl+Y` | Copy in Claude-friendly format |
| `Ctrl+G` | Select git changed files |
| `Ctrl+Shift+T` | Select test file pair |

## Auto-init Claude Config

```bash
# Add fileview MCP entry to ~/.claude.json
fv init claude

# Specify config path
fv init claude --path ~/.claude.json
```

## MCP Server Setup (Manual)

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

### Available MCP Tools

**File**: `list_directory`, `get_tree`, `read_file`, `read_files`, `write_file`, `delete_file`, `search_code`

**Git**: `get_git_status`, `get_git_diff`, `git_log`, `stage_files`, `create_commit`

**Analysis**: `get_file_symbols`, `get_definitions`, `get_references`, `get_diagnostics`

**Dependencies**: `get_dependency_graph`, `get_import_tree`, `find_circular_deps`

**Context**: `get_smart_context`, `estimate_tokens`, `compress_context`

**Project**: `run_build`, `run_test`, `run_lint`, `get_project_stats`
