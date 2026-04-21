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

## Live AI Activity Reflection (v2.5.0+)

When an AI agent calls `fv --mcp-server` in one process and you are running
the interactive `fv` in another, the interactive TUI surfaces the AI's tool
calls in real time — a status-bar indicator and an optional follow-mode that
auto-focuses the tree on the file the AI just touched.

### Setup

1. Make sure you are on `fileview >= 2.5.0` (`fv --version`).
2. Register `fv --mcp-server` with Claude Code. The easiest way is
   `fv init claude`, which edits `~/.claude.json` in place. The resulting
   entry looks like:

   ```json
   {
     "mcpServers": {
       "fileview": {
         "command": "fv",
         "args": ["--mcp-server", "/absolute/path/to/your/project"]
       }
     }
   }
   ```

3. In a separate terminal (attached to the same machine, same user), open the
   interactive fileview on the project root Claude Code was registered with:

   ```bash
   cd /absolute/path/to/your/project
   fv --follow-ai .
   ```

   Without `--follow-ai`, the TUI still shows the AI's activity in the status
   bar but does not auto-move focus.

4. When Claude Code runs a tool call via the MCP server (for example while
   reading `src/auth.rs`), the interactive fileview shows:

   ```
   [AI*] claude: read_file src/auth.rs
   ```

   and, with follow-mode on, reveals and focuses that path in the tree.

### Keybindings

| Key | Action |
|-----|--------|
| `Alt+A` | Toggle follow-mode (auto-focus on the AI's most recent file) |
| `Alt+L` | Open the live activity log popup (`j`/`k` to navigate, `Enter` to jump, `Esc`/`q` to close) |

Follow-mode is suppressed automatically while you are typing in any input
mode (search, filter, rename, bulk rename, fuzzy finder, confirmation), so it
never eats a keystroke you intended to land somewhere else.

### How the two processes talk

The MCP server and the interactive TUI are independent OS processes. They
rendezvous through a file-based protocol in your user cache directory:

- The interactive process registers a directory at
  `~/.cache/fileview/sessions/<pid>/` containing a `session.json`
  (pid + root + started_at) and an append-only `activity.jsonl`
  (permissions `0600` on unix).
- On every tool call, `fv --mcp-server` appends a JSONL line to each alive
  session whose root is an ancestor of the path being acted on.
- The interactive process watches the log via the `notify` crate and
  drains events per frame.

### Troubleshooting

- **Nothing shows up in the status bar.** Confirm the interactive `fv` is
  rooted at the same directory you passed to `fv --mcp-server`, or one of
  its ancestors. Events are scoped by path to keep unrelated projects
  from cross-talking.
- **`--follow-ai` printed "activity registry unavailable".** The cache
  directory could not be created (e.g. read-only home). Live reflection
  will still attempt events; only follow-mode is disabled.
- **Stale session directories.** The MCP server prunes sessions whose PID
  is no longer alive on the next emit. You can also remove
  `~/.cache/fileview/sessions/*` by hand — the interactive fileview will
  recreate its own entry on startup.
