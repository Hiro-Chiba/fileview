# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![Downloads](https://img.shields.io/crates/d/fileview.svg)](https://crates.io/crates/fileview)
[![CI](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml/badge.svg)](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.75-blue.svg)](https://www.rust-lang.org)

> Zero-config terminal file browser with automatic image preview

English | [日本語](README_ja.md)

## Why fv?

```
Lightweight ◄───────────────────────► Feature-rich

   nnn    lf    fv    ranger    yazi
  3.4MB  12MB  8MB    28MB     38MB
```

- **Zero config** - Install and run. No setup required.
- **Auto image preview** - Detects Kitty/iTerm2/Sixel/Halfblocks automatically
- **Fast** - 2.3ms startup, 8MB memory (vs ranger 400ms/28MB)
- **Batteries included** - Git status, syntax highlighting, PDF preview
- **Vim keybindings** - Navigate with j/k/h/l

## Quick Start

```bash
cargo install fileview
fv
```

## 3-Min AI Workflow (Recommended)

```bash
# 1) Set up Claude MCP integration once
fv init claude

# 2) Generate compact context for AI
fv --context-pack review --agent claude

# 3) Resume previous AI session context
fv --resume-ai-session
```

Useful docs:
- [Claude Code Guide](docs/CLAUDE_CODE.md)
- [Competitive Scorecard (weekly)](docs/COMPETITIVE_SCORECARD.md)
- [Release Policy](docs/RELEASE_POLICY.md)

## Features

| Feature | Description |
|---------|-------------|
| Tree navigation | Expand/collapse with vim keys |
| Multi-select | Batch operations on files |
| Preview panel | Text, images, archives, hex dump |
| File operations | Create, rename, delete, copy/paste |
| Fuzzy finder | `Ctrl+P` for quick search |
| Mouse support | Click, scroll, drag |
| Nerd Fonts | Icons enabled by default |

## Image Preview

FileView auto-detects your terminal:

| Terminal | Protocol |
|----------|----------|
| Kitty / Ghostty / Konsole | Kitty Graphics |
| iTerm2 / WezTerm / Warp | iTerm2 Inline |
| Foot / Windows Terminal | Sixel |
| VS Code / Alacritty | Halfblocks |

## Keybindings (Quick Reference)

| Key | Action |
|-----|--------|
| `j/k` | Navigate up/down |
| `h/l` | Collapse/expand |
| `g/G` | Top/bottom |
| `Space` | Toggle mark |
| `/` | Search |
| `Ctrl+P` | Fuzzy finder |
| `P` | Preview panel |
| `Ctrl+Shift+Enter` | Copy review context pack |
| `q` | Quit |

**[Full keybinding list](docs/KEYBINDINGS.md)**

## Claude Code Integration

FileView is the **only terminal file manager with native AI tooling support**.

```bash
# Project context for AI
fv --context

# Output directory tree
fv --tree --depth 2 ./src

# Quick file selection
selected=$(fv --select-mode --multi)

# Copy in Claude-friendly format (Ctrl+Y in fileview)
```

### Smart Selection

| Key | Action |
|-----|--------|
| `Ctrl+G` | Select all git changed files |
| `Ctrl+T` | Select test file pair |

### MCP Server

Use FileView as a Claude Code MCP server with Git integration:

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

**MCP 2.0 Tools (21 tools):**

| Category | Tools |
|----------|-------|
| File | `list_directory`, `get_tree`, `read_file`, `read_files`, `write_file`, `delete_file`, `search_code` |
| Git | `get_git_status`, `get_git_diff`, `git_log`, `stage_files`, `create_commit` |
| Analysis | `get_file_symbols`, `get_definitions`, `get_references`, `get_diagnostics` |
| Dependency | `get_dependency_graph`, `get_import_tree`, `find_circular_deps` |
| Context | `get_smart_context`, `estimate_tokens`, `compress_context` |
| Project | `run_build`, `run_test`, `run_lint`, `get_project_stats` |

**[Full MCP documentation](docs/CLAUDE_CODE.md)**

## CLI Options

```bash
fv [OPTIONS] [PATH]

Options:
  -p, --pick          Pick mode: output selected path(s)
  -f, --format FMT    Output format: lines, null, json
  --stdin             Read paths from stdin
  --on-select CMD     Run command on selection
  --choosedir         Output directory on exit
  --no-icons          Disable Nerd Fonts icons

Claude Code:
  -t, --tree          Output directory tree to stdout
  --depth N           Limit tree depth
  --context           Output project context (AI-friendly)
  --context-pack P    Output context pack preset (minimal/review/debug/refactor/incident/onboarding)
  --context-format F  Context pack format: ai-md, jsonl
  --agent A           Agent profile: claude, codex, cursor
  --token-budget N    Context pack token budget
  --include-git-diff  Force include git diff summary in context pack
  --include-tests     Include inferred test files in context pack
  --context-depth N   Fallback file scan depth for context pack
  --with-content      Include file contents in output
  --select-mode       Simple selection mode
  --multi             Allow multiple selection
  --select-related F  Output related files for a focused file
  --explain-selection Include score/reasons in related-file output
  --resume-ai-session [NAME]
                      Restore named AI session metadata (default: ai)
  --mcp-server        Run as MCP server
  benchmark ai        Run AI benchmark scenarios (use --scenario and --iterations)
  init claude         Initialize Claude config with fileview MCP entry
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Cancelled (pick mode) |
| 2 | Runtime error |
| 3 | Invalid arguments |

## Shell Integration

```bash
# Add to .bashrc or .zshrc
fvcd() {
  local dir=$(fv --choosedir "$@")
  [ -n "$dir" ] && [ -d "$dir" ] && cd "$dir"
}
```

## Installation Options

```bash
# Standard install
cargo install fileview

# With Chafa support (better image quality on basic terminals)
brew install chafa  # or apt install libchafa-dev
cargo install fileview --features chafa
```

## Stability

- Current channel: `stable` (`2.1.0`)
- Stable promotion criteria are documented in `docs/STABILITY.md`.
- As of 2026-02-04, criteria were satisfied and stable release was approved.

## Lua Plugin System

Extend FileView with Lua scripts:

```lua
-- ~/.config/fileview/plugins/init.lua

-- Startup notification
fv.notify("Plugin loaded!")

-- React to events
fv.on("file_selected", function(path)
    if path and path:match("%.secret$") then
        fv.notify("Warning: Secret file!")
    end
end)

-- Custom command
fv.register_command("copy-path", function()
    local file = fv.current_file()
    if file then
        fv.set_clipboard(file)
        fv.notify("Copied: " .. file)
    end
end)
```

**[Plugin API Reference](docs/PLUGINS.md)**

## Documentation

- [Claude Code Integration](docs/CLAUDE_CODE.md) - AI pair programming guide
- [Keybindings](docs/KEYBINDINGS.md) - Complete keybinding reference
- [Plugins](docs/PLUGINS.md) - Lua plugin system
- [Comparison](docs/COMPARISON.md) - vs yazi, lf, ranger, nnn
- [Roadmap](docs/ROADMAP.md) - Product direction and release history
- [Benchmarks](docs/BENCHMARKS.md) - Performance data
- [Security](docs/SECURITY.md) - Security model
- [Stability](docs/STABILITY.md) - Release channel policy and alpha exit criteria
- [Release Policy](docs/RELEASE_POLICY.md) - Versioning, cadence, and promotion rules

## License

MIT
