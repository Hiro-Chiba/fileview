# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.7.1] - 2026-05-16

Pure-refactor and release-tooling release. No behaviour, API, dependency,
binary-size, or feature changes.

### Internal

- Split `src/handler/action/tests.rs` (2928 LOC, one file) into a
  `tests/` directory with one file per pre-existing logical section
  (`basic`, `state_transition`, `sequence`, `edge_cases`, `focus`,
  `scroll_bounds`) plus a `mod.rs` that owns the shared helpers and the
  `call_handle_action!` macro.
- Split `src/render/status.rs` (1644 LOC, one file) into a `status/`
  directory: `bar.rs` (density variants + budget segment), `format.rs`
  (file metadata + size/time formatters), `popup.rs` (input/confirm/mini
  popups), `help.rs` (help overlay), `todos.rs` (TODO/FIXME popup), with
  `mod.rs` re-exporting the existing public renderers.

### Fixed

- `release.yml`: the awk pattern range used to extract the per-version
  changelog section was collapsing to one line because the start header
  also matched the end pattern. Replaced with a state-machine scan so
  future tag pushes get the right notes auto-attached to the GitHub
  Release.

## [2.7.0] - 2026-05-16

Three small non-interactive flags that let AI agents and shell scripts
talk to fileview without spinning up the TUI or the MCP server.

### Added

- `fv --tokens <path>` prints a cl100k_base token estimate for the file
  to stdout and exits. Useful for budgeting context before assembling a
  prompt. Falls back to the `chars / 4` heuristic when the `ai` feature
  is off, matching the rest of the codebase.
- `fv --snapshot-create <name>` captures a working-tree manifest
  (path / size / mtime, dotfiles skipped) to
  `.fileview/snapshots/<name>.json`. `fv --snapshot-diff <name>` then
  prints `+ added`, `- removed`, and `M modified` lines for every change
  since the snapshot. Lets a long-running agent answer "what have I
  touched since this session started" without committing intermediate
  state.
- `fv --watch <path> [--watch-timeout-secs N]` blocks until the path is
  modified externally, prints the changed paths, and exits. Exit code
  `0` on change, `1` on timeout. Drains the initial filesystem-event
  burst (notably macOS FSEvents replays) before waiting so a freshly
  created file does not register as an immediate "change".

### Internal

- New module `integrate::snapshot` with five unit tests covering
  capture, diff, save/load round-trip, dotfile skipping, and missing
  snapshots.
- New module `integrate::watch` with four unit tests covering the
  empty-paths guard, the missing-path guard, the timeout path, and the
  modification path.
- E2E coverage: 5 tests for `--tokens`, 8 for snapshot, 4 for watch
  (21 new tests in `tests/e2e/`).

## [2.6.0] - 2026-05-11

A focused release on AI-driven workflows and slim builds. Adds eight
user-facing capabilities and four optional Cargo features so users can
strip dependencies they do not use.

### Added

- **AI ignore synthesizer** (`fv init aiignore`): generates
  `.claudeignore`, `.cursorignore`, and `.aiderignore` with a curated
  skip list. Re-running rewrites only the `# === fv-managed start ===`
  block, keeping user rules outside it intact. (#190)
- **Context budget bar**: the status bar now shows a live token
  estimate for the marked file set and percentage of the chosen
  model's context window (e.g. `[ctx 12f 8.4k/200k S4.6 4%]`). Color
  switches from green to yellow at 50%, red at 80%, magenta when
  over. `Alt+M` cycles through Sonnet 4.6 200k / 1M, Opus 4.7,
  Haiku 4.5, and GPT-4o 128k. Persisted via
  `[budget].default_model` in `config.toml`. (#192)
- **Repo fingerprint**: a one-line repo summary surfaces in the
  status bar shortly after startup
  (`Rust workspace · 4 crates · 12k LOC · last touched 14m ago`).
  Detection runs on a background thread bounded by a 2 s budget. (#193)
- **Diff-aware tree** (`fv --diff [REVSPEC]`): each file in scope
  shows a `+N/-M` annotation in green and red. With no argument the
  scope is the working tree (combined unstaged + staged + untracked);
  with a revspec like `origin/main..HEAD` it is that range. (#194)
- **TODO/FIXME aggregator** (`\` key): popup that lists every TODO,
  FIXME, XXX, HACK, BUG, NOTE marker found in source files under the
  current root. `j`/`k` move, `Enter` jumps, `Esc` closes. Scan is
  bounded by a 1500 ms time budget. (#195)
- **AI session replay** (`fv --replay [ID]`): two-pane TUI that
  lists past `fv --mcp-server` sessions and lets you scrub through
  every recorded `activity.jsonl` event. `Enter` exits cleanly and
  prints the focused event's path to stdout for shell integration. (#196)

### Changed

- **Cold-cache warmup**: a detached background thread is spawned at
  `run_app` entry to warm syntect's grammar / theme load and
  tiktoken's `cl100k_base` BPE table. The first preview, diff render,
  and budget update no longer pay the 30 to 80 ms first-call cost on
  the UI thread. (#198)
- **Release notes format**: `docs/RELEASE_POLICY.md` now codifies
  the GitHub Releases body format (English, Keep a Changelog
  sections, no decorative emojis). All historical milestone releases
  were rewritten to match. (#191)

### Build

- **Feature flags**: introduced `ai`, `archive`, `clipboard`, and
  `lua` Cargo features, all on by default. `cargo install fileview
  --no-default-features` produces a slim binary that drops
  `tiktoken-rs`, `petgraph`, `zip`, `tar`, `flate2`, `arboard`, and
  `mlua` (vendored Lua 5.4). Each feature is independently
  togglable. CI now lints both default and `--no-default-features`
  builds on every PR. (#197, #199, #200, #201)
- When `ai` is off, `estimate_tokens` falls back to a `chars / 4 + 1`
  heuristic so the budget bar and context-pack truncation keep
  working at reduced precision.
- When `clipboard` is off, `c` and `Ctrl+Y` surface
  "clipboard support disabled" instead of writing to the OS
  clipboard.
- When `lua` is off, `~/.config/fileview/plugins/init.lua` is no
  longer auto-loaded; `fv plugin test` returns the disabled error.
- When `archive` is off, the preview pane shows
  "archive support disabled" instead of zip / tar.gz contents.

### Notes

- The `image`, `pdf`, and `video` previews remain always-on. They
  are tracked as a follow-up because making them optional requires
  a larger refactor of the preview type definitions.

## [2.5.0] - 2026-04-19

### Added

- **Live AI activity reflection**: when an AI agent (e.g. Claude Code) calls
  `fv --mcp-server` in a separate process, the interactive `fv` TUI now shows
  the AI's tool calls in real time. The most recent event appears in the
  status bar as `[AI] claude: read_file src/auth.rs`, and an optional
  follow-mode auto-focuses the tree on the file the AI just touched. (#179)
  - `Alt+A`: toggle follow-mode (opt-in; off by default so the UI never
    interrupts text input unexpectedly).
  - `Alt+L`: open the live activity log popup (up to 100 events, `j/k`
    navigation, `Enter` to jump to the path, `Esc`/`q` to close).
  - `--follow-ai` CLI flag to start with follow-mode enabled.
  - File-based protocol: the MCP server writes events to
    `~/.cache/fileview/sessions/<pid>/activity.jsonl`; the interactive TUI
    watches that file via `notify`. The log is owner-only (`0600`) on unix.
  - Batch tools (`read_files`, `estimate_tokens`) emit one event per path so
    every file the AI touches is visible in the UI.

## [2.4.1] - 2026-04-19

### Changed

- **CI pipeline streamlined**: reduced required jobs per PR from 11 to 6.
  `fmt` / `clippy` / `rustdoc` now share a single `lint` job, `audit` and
  `coverage` moved to scheduled + main push, MSRV check is `cargo check
  --locked` only. (#176)
- **Crate description** reworded from "A minimal file tree UI for terminal
  emulators" to "A fast, keyboard-driven TUI file browser with previews
  and git" to reflect the actual feature surface. (#177)
- Updated `zip` dependency to 8.3.1 (major version bump, internal to
  archive preview; no user-visible behavior change). (#172)
- Refreshed rust-minor-patch group and CI action versions
  (codecov-action v6, action-gh-release v3). (#171, #173, #174)

### Fixed

- Resolved new Clippy lints surfaced by Rust 1.95. (#175)
- Security audit CI job failed with `Resource not accessible by
  integration`; granted `checks: write` and `issues: write` permissions
  to `rustsec/audit-check`. (#177)

### Docs

- Fact-check pass across README, DESIGN, BENCHMARKS, CLAUDE_CODE,
  keybindings and theme docs; several inaccuracies corrected.
- `docs/BENCHMARKS.md` comparison table: configuration file support is
  optional, not absent. (#177)
- `docs/DESIGN.md`: tech stack now lists `anyhow + thiserror`; Non-Goals
  clarifies archive creation/extraction is out of scope while read-only
  preview is supported. (#177)
- Refreshed `docs/STABILITY.md` assessment date. (#177)

## [2.4.0] - 2026-02-28

### Added

- Speed-oriented `release-fast` Cargo profile for maximum runtime performance

### Changed

- **Dirty-frame rendering**: skip redrawing on idle frames, reducing CPU usage when nothing changes
- **Cached visible entries**: flatten and cache the visible tree with a dirty flag, avoiding repeated traversal
- **Event-driven watcher sync**: sync file watcher with expanded directories only when the set changes
- **Cached status-bar metadata**: cache file size / mod-time to eliminate repeated `stat()` calls
- **Precomputed sort keys**: compute sort keys once on tree load instead of on every comparison
- **Async preview worker**: move heavy preview generation (syntax highlight, hex dump, etc.) off the UI thread
- **Incremental fuzzy narrowing**: when the query extends the previous prefix, re-scan only prior matches instead of all candidates
- Updated dependencies (petgraph 0.8.3, toml 1.0.1, rust-minor-patch group, Cargo.lock refresh)

## [2.3.2] - 2026-02-12

### Changed

- Updated README with missing CLI options (`--hidden`, `--session`, `--selection-path`, `plugin` commands)
- Added environment variables section to README
- Updated stability version to 2.3.2
- Resized and centered screenshot in Japanese README

## [2.3.1] - 2026-02-12

### Changed

- Updated dependencies via Dependabot (mlua, tiktoken-rs, zip, dirs, toml, notify-debouncer-mini, etc.)
- Updated CI actions (checkout v6, upload-artifact v6, download-artifact v7, codecov-action v5)
- Resized and centered screenshot in README

### Fixed

- Windows test compatibility improvements

## [2.3.0] - 2026-02-11

### Added

- Screenshot added to README for better project presentation
- Enhanced CI/CD pipeline with multi-platform builds, security audit, and code coverage

### Changed

- Updated MSRV to 1.90.0 (verified with dependency requirements)
- Improved GitHub Actions workflow with matrix builds (Linux, macOS, Windows)
- Added Dependabot configuration for automated dependency updates

### Note

- First public release on GitHub

## [2.2.3] - 2026-02-04

### Changed

- 開発/運用ルールを整理し、`main` を最終スナップショット向けのシンプル構成にクリーンアップ

### Note

- このリリースを区切りとして、以後の開発は一時停止


## [2.2.2] - 2026-02-04

### Changed

- Narrow UI (`25-39` columns) でも Nerd Fonts のファイル/フォルダアイコンを表示するよう改善
- ドキュメントの密度テーブルを実装仕様に合わせて更新


## [2.2.1] - 2026-02-04

### Added

- `init claude`: Claude設定への `fileview` MCPエントリ自動初期化
- `--resume-ai-session [NAME]`: 名前付きAIセッション復元（既定: `ai`）
- `Ctrl+Shift+Enter`: review context pack のクイックコピー
- `docs/CLAUDE_CODE_ja.md`: Claude連携ガイド（日本語）を追加

### Changed

- `docs/DEVELOPMENT_HISTORY.md` を最新リリース履歴（`v2.1.0` まで）に同期
- `docs/ROADMAP.md` を管理対象として整備
- `README.md` / `README_ja.md` のAI導線とリンクを更新

### Fixed

- `notify-types` ロック整合を修正し、公開フローを安定化

## [2.2.0] - 2026-02-04

### Note

- このタグは公開時のロック不整合対応のため、実運用上は `2.2.1` を最新安定版として利用してください。

## [2.1.0] - 2026-02-04

### Added

- stable昇格基準の文書化（`docs/STABILITY.md`）

### Changed

- バージョン表記を `2.1.0`（stable）へ移行

## [2.0.4-alpha] - 2026-02-04

### Changed

- stable昇格直前の最終調整

## [2.0.3-alpha] - 2026-02-04

### Added

- context-pack の agent/profile/options 拡張
- related-file選定・AI履歴メタデータ・AIベンチ機能を強化

## [2.0.2-alpha] - 2026-02-04

### Changed

- 2.x系の継続改善

## [2.0.1-alpha] - 2026-02-03

### Security

- **Command Injection Prevention** (`project.rs`)
  - Added command whitelist for `run_build` (cargo, npm, yarn, etc.)
  - Reject shell metacharacters in command arguments

- **DoS Prevention**
  - `project.rs`: Added depth limit (50) and file size limit (10MB) for `get_project_stats`
  - `file.rs`: Added maximum tree depth (20) for `get_tree`
  - `file.rs`: Added search pattern length limit (500 chars) for `search_code`

- **Path Traversal Prevention** (`git.rs`)
  - Added path validation to `git_log` and `stage_files`

- **Shell Injection Prevention** (`custom.rs`)
  - Added shell escaping for file paths in custom preview commands

### Fixed

- `server.rs`: Replaced `unwrap()` with proper error handling for JSON serialization
- `file.rs`: Reject null bytes in search patterns to prevent injection
- `token.rs`: Fixed escaped newline bug (\\n → \n)
- `analysis.rs`: Refactored to eliminate code duplication with generic `extract_symbols_with_patterns`
- `common.rs`: Added negative timestamp handling for pre-1970 dates

## [2.0.0-alpha] - 2026-02-03

### Added

- **Ultra-Narrow UI (20文字幅対応)**
  - `UiDensity` enum (Full/Compact/Narrow/Ultra) for responsive terminal adaptation
  - `LayoutEngine` for dynamic width-based layout computation
  - Tree rendering optimization for 20-24 character terminals
  - Icon hiding in Ultra/Narrow modes for space efficiency
  - Adaptive filename truncation based on available width

- **MCP 2.0: AI-Native Development Tools**
  - **Auto Tool Registration** (`registry.rs`): 21 tools across 6 categories
  - **Security Utilities** (`security.rs`): Path validation, sensitive file detection, filename sanitization
  - **Token Estimation** (`token.rs`): tiktoken-rs integration for AI context optimization
  - **Smart Context** (`get_smart_context`): AI-optimized context with dependency awareness
  - **Dependency Analysis** (`get_dependency_graph`, `get_import_tree`, `find_circular_deps`)
  - **Code Analysis** (`get_definitions`, `get_references`, `get_diagnostics`)
  - **Project Management** (`run_build`, `run_test`, `run_lint`, `get_project_stats`)

- **Unified Error Handling** (`src/error.rs`)
  - `FileviewError` enum with thiserror derivation
  - Consistent error types: IO, Git, MCP, Preview, Path, Render, Token, Analysis

### Changed

- **MCP Handler Restructure**: Split monolithic `handlers.rs` into focused modules
  - `handlers/file.rs`: File operations (read, write, delete, search)
  - `handlers/git.rs`: Git operations (status, diff, log, stage, commit)
  - `handlers/context.rs`: AI context generation
  - `handlers/analysis.rs`: Code analysis and symbols
  - `handlers/dependency.rs`: Dependency graph analysis
  - `handlers/project.rs`: Build, test, lint operations

- **Preview Module Split**: Refactored 1880-line `preview.rs` into 11 focused modules
  - `preview/common.rs`: Shared utilities
  - `preview/text.rs`: Syntax-highlighted text preview
  - `preview/hex.rs`: Binary file hex dump
  - `preview/archive.rs`: ZIP/tar.gz archive preview
  - `preview/pdf.rs`: PDF preview (pdftoppm)
  - `preview/image.rs`: Image preview (Sixel/Kitty/iTerm2)
  - `preview/video.rs`: Video thumbnail and metadata
  - `preview/diff.rs`: Git diff preview
  - `preview/custom.rs`: External command preview
  - `preview/directory.rs`: Directory info preview

### Dependencies

- Added `thiserror = "2"` for unified error types
- Added `petgraph = "0.6"` for dependency graph analysis
- Added `tiktoken-rs = "0.5"` for token estimation

## [1.26.0] - 2026-02-03

### Added

- **Ultra-compact status bar** for 30-40 character width terminals
  - Displays: help hint, selection count, git branch, sort mode, filter indicator
  - Format: `? 3* ⎇main S` for maximum information density
- **Auto-hide preview** when terminal width is below threshold (default: 50 chars)
  - New `auto_hide_preview_threshold` setting in AppState
  - `effective_preview_visible()` method for width-aware preview control
- **Peek mode** (`Alt+P`): Mini-preview in status bar area
  - Shows first few lines of file content in narrow terminals
  - `PreviewDisplayMode` enum (Normal/Peek)
- **Enhanced path abbreviation** in tree view
  - `<20` chars: filename only
  - `20-30` chars: last directory + filename
  - `>30` chars: single-character abbreviation for parent directories
- **MCP file operation tools**:
  - `write_file`: Create/update files with `create_dirs` option
  - `delete_file`: Delete files/directories (trash support, recursive option)
  - `read_files`: Batch read multiple files at once
- **MCP extended Git operations**:
  - `git_log`: Get commit history with optional path filter
  - `stage_files`: Stage files for commit
  - `create_commit`: Create commit with message
- **MCP symbol extraction** (`get_file_symbols`):
  - Regex-based symbol detection for Rust, Python, TypeScript, Go, Java/Kotlin
  - Returns functions, classes, structs, enums, interfaces, traits
- **Session persistence** (`--session save/restore/clear`):
  - Save selection state to `.fileview-session.json`
  - Restore previous session on startup
- **Extended smart selection keybindings**:
  - `Ctrl+1..9`: Select files by extension (.rs, .ts, .js, .py, .go, .java, .md, .json, .css)
  - `Alt+R`: Select files from most recent git commit
  - `Alt+g`: Select only git staged files
- **Compact copy format** (`Alt+Y`): Minimal header format for smaller AI contexts

### Dependencies

- Added `regex = "1"` for symbol extraction

### New Files

- `src/integrate/session.rs`: Session save/restore functionality

## [1.24.1] - 2026-02-03

### Added

- **docs/CLAUDE_CODE.md**: Comprehensive Claude Code integration guide
  - MCP server configuration examples
  - Workflow examples (code review, bug investigation, refactoring)
  - CLI reference and keybindings
- **AI Integration section in COMPARISON.md**: Unique differentiator scoring
  - fileview scores 10/10 in AI Integration (competitors: 0)
  - Updated total score: 72 → 82
- **Plugin examples** in `examples/plugins/`:
  - `claude_code.lua`: Claude Code integration plugin
  - `git_actions.lua`: Quick git operations
  - `productivity.lua`: General productivity commands

## [1.24.0] - 2026-02-03

### Added

- **Claude Code integration features**: Comprehensive AI pair programming support
  - **CLI output mode** (`--tree`, `-t`): Output directory tree to stdout (non-interactive)
    - `--depth N`: Limit tree traversal depth
    - `--with-content`: Include file contents in pick output (Claude-friendly format)
  - **Clipboard enhancements**:
    - `Y`: Copy file content to system clipboard
    - `Ctrl+Y`: Copy in Claude-friendly markdown format with syntax highlighting hints
  - **Interactive select mode** (`--select-mode`): Simplified file selection
    - Enter to select and output path to stdout immediately
    - `--multi`: Allow multiple selection before confirmation
  - **MCP server** (`--mcp-server`): JSON-RPC server for Claude Code integration
    - `list_directory`: List files and directories
    - `get_tree`: Get directory tree structure
    - `read_file`: Read file content
    - Protocol version: 2024-11-05

### New Files

- `src/integrate/tree.rs`: Tree output logic for CLI mode
- `src/mcp/mod.rs`: MCP module entry point
- `src/mcp/server.rs`: JSON-RPC server implementation
- `src/mcp/handlers.rs`: MCP tool handlers
- `src/mcp/types.rs`: MCP protocol type definitions

### Dependencies

- Added `serde_json = "1.0"` for MCP JSON-RPC serialization

### Example Usage

```bash
# Output directory tree
fv --tree --depth 2 ./src

# Copy files with content for Claude
fv --pick --with-content

# Use as MCP server in Claude Code
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | fv --mcp-server

# Quick file selection
selected=$(fv --select-mode --multi)
```

## [1.23.0] - 2026-02-03

### Added

- **Adaptive status bar for narrow terminals**: Status bar now adjusts layout based on screen width
  - Wide screens (≥100 chars): Full 50:50 split with all information
  - Medium screens (60-99 chars): Abbreviated display with 45:55 split
  - Narrow screens (<60 chars): Single panel with essential info only
  - Abbreviated labels: "Selected: 3" → "Sel:3", "Copied: 2" → "Cp:2"
  - Shortened time format: "2h ago" → "2h"
  - Compact help text: "? for help" → "?" on narrow screens

## [1.22.0] - 2026-02-03

### Added

- **Lua plugin loader integration (Phase 12d)**: Full plugin system integration
  - Plugins are automatically loaded from `~/.config/fileview/plugins/init.lua` at startup
  - Plugin events are fired automatically:
    - `start`: After plugins load
    - `file_selected`: When focused file changes
    - `directory_changed`: When navigating to a new directory
    - `selection_changed`: When multi-selection changes
    - `before_quit`: Before application exits
  - Plugin actions are processed each frame:
    - `fv.navigate(path)`: Navigate to directory
    - `fv.select(path)` / `fv.deselect(path)`: Modify selection
    - `fv.clear_selection()`: Clear all selections
    - `fv.refresh()`: Reload tree view
    - `fv.set_clipboard(text)`: Set clipboard content
    - `fv.focus(path)`: Focus on specific file
  - Plugin notifications are displayed in the status bar

### Example Plugin

```lua
-- ~/.config/fileview/plugins/init.lua

-- Show notification on startup
fv.notify("Plugin loaded! FileView v" .. fv.version())

-- React to file selection
fv.on("file_selected", function(path)
    if path and path:match("%.secret$") then
        fv.notify("Warning: Secret file!")
    end
end)

-- Custom command to copy file path
fv.register_command("copy-path", function()
    local file = fv.current_file()
    if file then
        fv.set_clipboard(file)
        fv.notify("Copied: " .. file)
    end
end)
```

## [1.21.0] - 2026-02-03

### Added

- **Lua plugin registration API (Phase 12c)**: Register custom commands, event handlers, and previewers
  - `fv.register_command(name, fn)`: Register a custom command callable from FileView
  - `fv.on(event, fn)`: Register event handlers for FileView events
    - Supported events: `file_selected`, `directory_changed`, `selection_changed`, `start`, `before_quit`
    - Multiple handlers can be registered for the same event
  - `fv.register_previewer(pattern, fn)`: Register custom file previewers by glob pattern
    - Pattern supports `*` (any characters) and `?` (single character)
    - Returns preview content as string
  - Internal storage tables: `fv._commands`, `fv._events`, `fv._previewers`

### New Methods (PluginManager)

- `has_command(name)`: Check if command is registered
- `list_commands()`: List all registered command names
- `invoke_command(name)`: Execute a registered command
- `fire_event(event, arg)`: Fire event and call all handlers
- `has_previewer(pattern)`: Check if previewer is registered
- `list_previewers()`: List all registered previewer patterns
- `find_previewer(filename)`: Find matching previewer for filename
- `invoke_previewer(pattern, path)`: Execute previewer and get result

### Tests

- Added 20 new unit tests for registration API
- Total plugin tests: 50

## [1.20.0] - 2026-02-03

### Added

- **Lua plugin action API (Phase 12b)**: Plugins can now trigger FileView actions
  - `fv.navigate(path)`: Navigate to a directory
  - `fv.select(path)`: Add a file to selection
  - `fv.deselect(path)`: Remove a file from selection
  - `fv.clear_selection()`: Clear all selections
  - `fv.refresh()`: Refresh the tree view
  - `fv.set_clipboard(text)`: Set clipboard text
  - `fv.focus(path)`: Focus on a specific file (reveal and select)
  - Actions are queued and processed by the event loop

### Changed

- `PluginContext` now includes action queue for plugin-triggered actions
- `PluginManager` collects actions alongside notifications

### Tests

- Added 15 new unit tests for action API (12 lua + 3 api)
- Total plugin tests: 30

## [1.19.0] - 2026-02-03

### Added

- **Lua plugin system (Phase 12a)**: Scripting support via Lua 5.4
  - Plugin location: `~/.config/fileview/plugins/init.lua`
  - Uses `mlua` crate with vendored Lua 5.4
  - Read-only API for accessing FileView state:
    - `fv.current_file()`: Get currently focused file path
    - `fv.current_dir()`: Get current directory path
    - `fv.selected_files()`: Get list of selected files
    - `fv.notify(msg)`: Display notification message
    - `fv.version()`: Get FileView version
    - `fv.is_dir(path)`: Check if path is directory
    - `fv.file_exists(path)`: Check if path exists

### New Files

- `src/plugin/mod.rs`: Plugin system module entry (~30 lines)
- `src/plugin/lua.rs`: Lua runtime management and API setup (~280 lines)
- `src/plugin/api.rs`: Plugin context for state sharing (~100 lines)

### Dependencies

- Added `mlua = { version = "0.10", features = ["lua54", "vendored"] }`

### Tests

- Added 18 new unit tests for plugin module
- Total test count: 436 tests (346 unit + 72 E2E + 18 plugin)

### Documentation

- Added `docs/ROADMAP_V4.md`: Future development roadmap for v4+ features

## [1.18.0] - 2026-02-03

### Added

- **Video preview**: Preview video files with thumbnails and metadata
  - Supported formats: mp4, mkv, webm, avi, mov, wmv, flv, m4v
  - Displays thumbnail extracted via ffmpeg (1 second position)
  - Shows metadata: duration, resolution, codec, audio codec, bitrate, frame rate
  - Graceful fallback when ffmpeg/ffprobe is not installed
- **Tab support**: Full tab integration for multiple directories
  - `Ctrl+T`: Open new tab in current directory
  - `Ctrl+W`: Close current tab
  - `Alt+t`: Switch to next tab
  - `Alt+T`: Switch to previous tab
  - Tab bar displayed when multiple tabs are open
  - Each tab maintains independent state (navigation, selection, preview)
- **Event hooks system**: Execute scripts on file operations
  - Configure in `~/.config/fileview/config.toml` under `[hooks]` section
  - Available hooks: `on_create`, `on_delete`, `on_rename`, `on_cd`, `on_start`, `on_exit`
  - Environment variables: `$FILEVIEW_PATH`, `$FILEVIEW_OLD_PATH`, `$FILEVIEW_DIR`, `$FILEVIEW_SELECTED`
  - Supports tilde expansion in script paths
- **Shell integration**: Enhanced shell integration options
  - `--choosedir [FILE]`: Write directory path to file on exit (for cd integration)
  - `--selection-path FILE`: Write selected file paths to file on exit
  - `Alt+S`: Open subshell in current directory (returns to fileview on exit)
  - Added shell function example in help text for automatic cd on exit
- **Batch operations**: Enhanced multi-file selection
  - `V`: Enter visual select mode (vim-like range selection)
  - `*`: Select all / deselect all (toggle)
  - `Alt+i`: Invert selection
  - Visual select mode: j/k extends selection, y/d/D operates on selection

### New Files

- `src/app/video.rs`: Video metadata extraction and thumbnail generation (~200 lines)
- `src/handler/hooks.rs`: Event hooks configuration and execution (~150 lines)

### Changed

- `src/handler/action/navigation.rs`: Navigation now updates selection range in visual select mode
- `src/handler/action/mod.rs`: Added handlers for visual select, select all, invert selection
- `src/handler/action/selection.rs`: Added `handle_with_entries()` and `select_range()` functions
- `src/handler/key.rs`: Added `handle_visual_select_mode()` function
- `src/core/mode.rs`: Added `ViewMode::VisualSelect` variant
- `src/app/event_loop.rs`: Integrated TabManager for full tab support
- `src/app/render.rs`: Added tab bar rendering
- `src/app/config.rs`: Added `--choosedir` and `--selection-path` CLI options
- `src/tree/navigator.rs`: Added `Clone` derive for tab state copying

### Tests

- Added 6 new unit tests for video module (metadata parsing, file detection)
- Added 6 new unit tests for hooks module (config, environment, script expansion)
- Total test count: 434 tests (431 passing, 3 ignored)

## [1.17.0] - 2026-01-31

### Added

- **Custom commands system**: Define and execute shell commands via config
  - Commands section in `config.toml`: `[commands]`
  - Placeholder expansion: `$f` (file path), `$d` (directory), `$n` (filename), `$s` (stem), `$e` (extension), `$S` (selected files)
  - Keymap binding with `command:name` syntax
  - Example: `open = "open $f"`, `edit = "nvim $f"`
- **Custom preview system**: External scripts for file preview
  - Custom preview section in `config.toml`: `[preview.custom]`
  - Extension-to-command mapping: `md = "glow -s dark $f"`, `json = "jq -C . $f"`
  - Scroll support for custom preview output
- **Documentation**: Comprehensive configuration guides
  - `docs/CONFIGURATION.md`: Main configuration reference
  - `docs/THEMES.md`: Theme customization guide
  - `docs/COMMANDS.md`: Custom commands guide
  - `examples/config.toml`, `examples/keymap.toml`, `examples/theme.toml`: Example files

### New Files

- `src/handler/action/command.rs`: Command execution logic (~120 lines)
- `src/render/preview.rs`: Added `CustomPreview` struct and `render_custom_preview` function
- `docs/CONFIGURATION.md`: Configuration guide
- `docs/THEMES.md`: Theme customization guide
- `docs/COMMANDS.md`: Custom commands guide
- `examples/`: Example configuration files

### Changed

- `PreviewState` now includes custom preview support
- `handle_action` signature updated to include custom preview parameter
- `handle_preview_scroll` now handles custom preview scrolling
- Added `Clone` derive to `CommandsConfig` and `PreviewConfig`
- Exported `CommandsConfig` and `PreviewConfig` from `app` module

### Tests

- Added 16 new integration tests for custom commands and custom preview
- Added 6 new E2E tests for config file parsing with new features
- Total test count: 346 unit/integration tests + 72 E2E tests = 418 tests

## [1.16.0] - 2026-01-31

### Added

- **Configuration file system**: Full customization via `~/.config/fileview/config.toml`
  - General settings: `show_hidden`, `enable_icons`, `mouse_enabled`
  - Preview settings: `hex_max_bytes`, `max_archive_entries`, `image_protocol`
  - Performance settings: `git_poll_interval_secs`
  - UI settings: `show_size`, `show_permissions`, `date_format`
- **Keymap customization**: Custom key bindings via `~/.config/fileview/keymap.toml`
  - Mode-based keybindings (browse, preview, search, confirm, fuzzy, help, filter)
  - Support for modifier keys (Ctrl, Alt, Shift)
  - Dynamic dispatch with `KeyBindingRegistry`
- **Theme system**: Color customization via `~/.config/fileview/theme.toml`
  - Base colors: background, foreground, selection, border, status bar
  - File type colors: directory, executable, symlink, archive, image, etc.
  - Git status colors: modified, staged, untracked, deleted, renamed, conflict
  - Color formats: named colors, hex (#rgb, #rrggbb), rgb(), 256-color index
- **CLI options**: Added `--hidden` / `-a` and `--no-hidden` flags

### New Files

- `src/app/config_file.rs`: Configuration file loading and parsing (~130 lines)
- `src/handler/keymap.rs`: Keymap configuration and registry (~450 lines)
- `src/render/theme.rs`: Theme configuration and color management (~470 lines)
- `tests/e2e/config_cli.rs`: E2E tests for config CLI options (16 tests)

### Changed

- CLI arguments take precedence over config file settings
- Tree and status bar now use theme colors instead of hardcoded values
- Total test count: 433 → 804 tests (+371 tests including 44 new integration tests)

### Dependencies

- Added `toml = "0.8"` for TOML parsing
- Added `dirs = "5.0"` for config directory detection
- Added `serde = { version = "1.0", features = ["derive"] }` for deserialization

## [1.15.1] - 2026-01-31

### Added

- **Comprehensive test suite**: Added 93 new tests (80 unit tests + 13 e2e tests)
  - `src/render/bulk_rename.rs`: Layout calculation tests for `centered_rect()`
  - `src/handler/action/bulk_rename.rs`: Pattern matching and buffer update tests
  - `src/core/tab.rs`: Tab management and navigation tests
  - `src/git/diff.rs`: Diff parsing tests for multiple hunks, edge cases
  - `src/git/operations.rs`: Real git repository stage/unstage tests
  - `src/handler/action/git_ops.rs`: Action handler tests
  - `tests/e2e/git_ops.rs`: Git repository e2e tests
  - `tests/e2e/bulk_rename.rs`: File operations e2e tests

### Changed

- Total test count: 340 → 433 tests

## [1.15.0] - 2026-01-31

### Added

- **Async image loading**: Background image loading using std::thread + mpsc channels prevents UI freezing on large images
- **Git stage/unstage**: Press `s` to stage files, `u` to unstage files with visual indicators (`+` staged, `~` modified)
- **Git diff preview**: Color-coded diff display for modified files (green for additions, red for deletions)
- **Bulk rename**: Press `R` with multiple files selected for pattern-based renaming (supports wildcards like `*.txt` → `*.md`)
- **Tab support (foundation)**: Tab and TabManager structures with key bindings (`Ctrl+T`, `Ctrl+W`, `Alt+t`, `Alt+T`)

### New Files

- `src/app/image_loader.rs`: Background image loader
- `src/git/operations.rs`: Git stage/unstage operations
- `src/git/diff.rs`: Git diff parsing
- `src/handler/action/git_ops.rs`: Git action handler
- `src/handler/action/bulk_rename.rs`: Bulk rename logic
- `src/render/bulk_rename.rs`: Bulk rename dialog rendering
- `src/core/tab.rs`: Tab and TabManager structures
- `src/render/tabs.rs`: Tab bar rendering

## [1.14.4] - 2026-01-31

### Documentation

- **README**: Added positioning diagram and competitive comparison
- **README_ja**: Updated Japanese README with same improvements, fixed MSRV badge
- **BENCHMARKS.md**: Added performance comparison table with yazi, lf, ranger, nnn
- **COMPARISON.md**: New comprehensive comparison guide with feature matrix

## [1.14.3] - 2026-01-31

### Security

- **Archive DoS prevention**: Added 4KB limit for archive entry names to prevent memory exhaustion from malicious archives

### Changed

- **Code refactoring**: Extracted `get_border_style()` helper to eliminate 6 duplicated border styling blocks
- **Code refactoring**: Added `truncate_entry_name()` helper for consistent entry name handling

## [1.14.2] - 2026-01-31

### Fixed

- **tar.gz date display**: Fixed incorrect date calculation in archive preview that didn't account for leap years and month lengths
- **Bookmark key handling**: Replaced `unwrap()` with safe pattern matching for bookmark slot keys ('1'-'9')

## [1.14.1] - 2026-01-31

### Changed

- **MSRV badge**: Updated README badge from 1.70 to 1.75 to match actual requirement
- **ROADMAP.md**: Updated current status to v1.14.0, added v1.9.0-v1.14.0 features

### Fixed

- **Clippy warnings**: Fixed all clippy warnings for cleaner builds
  - `src/render/fuzzy.rs`: Use `const { assert!(..) }` for compile-time checks
  - `tests/integration.rs`: Remove redundant `.clone()` calls on Copy types
  - `tests/integration.rs`: Use `!is_empty()` instead of `len() >= 1`
  - `tests/integration.rs`: Remove redundant single-component imports
  - `tests/e2e/*.rs`: Use `cargo_bin_cmd!` macro instead of deprecated `cargo_bin`

### Other

- **.gitignore**: Added patterns for IDE files, OS files, and misc temp files

## [1.14.0] - 2026-01-31

### Added

- **Syntax highlighting**: Code files now display with syntax highlighting
  - Uses `syntect` crate (same engine as bat/delta)
  - Supports 100+ languages: Rust, Python, JavaScript, TypeScript, Go, C/C++, Java, Ruby, PHP, Swift, Kotlin, Scala, HTML, CSS, JSON, YAML, TOML, XML, Markdown, SQL, Shell/Bash, and more
  - Theme: base16-ocean.dark (terminal-compatible)
  - Language detection via file extension or shebang
  - Falls back to plain text for unsupported languages

### Dependencies

- Added `syntect` crate (v5) for syntax highlighting

## [1.13.0] - 2026-01-31

### Added

- **PDF preview**: Preview PDF files as images using poppler-utils
  - Renders pages at 150 DPI for good quality
  - Navigate pages with `[` (previous) and `]` (next) keys
  - Page info displayed in title bar: `document.pdf (3/10) [/] prev/next`
  - Falls back to hex preview if pdftoppm is not installed
  - Temporary files auto-cleaned via tempfile crate

### Dependencies

- Added `tempfile` crate (v3) for temporary file management

## [1.12.0] - 2026-01-31

### Security

- **stdin path traversal fix**: `--stdin` mode now validates paths using `canonicalize()` to prevent path traversal attacks (e.g., `../../../etc/passwd`)
- **git command PATH hardening**: Git executable is now resolved from standard system paths (`/usr/bin/git`, `/usr/local/bin/git`, `/opt/homebrew/bin/git`) before falling back to `$PATH`, preventing malicious git binary injection
- **TOCTOU mitigation**: `get_unique_path()` now uses bounded counter (1-1000) with timestamp fallback to reduce race condition window during file copy operations

### Added

- **Security documentation**: New `docs/SECURITY.md` documenting security model and `--on-select` callback safety guidelines

## [1.11.3] - 2026-01-31

### Added
- **E2E tests**: Expanded end-to-end test suite (16 new tests, 37 total)
  - Valid path input tests: absolute paths, file paths, nested directories
  - Multiple paths mode: directory and file combinations
  - Pick mode tests: flag acceptance, format options, on-select command
  - Pick mode combinations: icons, multiple options

## [1.11.2] - 2026-01-31

### Added
- **Unit tests**: Added comprehensive tests for mouse module (29 new tests)
  - ClickDetector: double-click detection, reset behavior, row tracking
  - PathBuffer: buffer operations, path parsing, clear/take functionality
  - normalize_shell_path: edge cases, special characters, URL encoding
  - parse_paths: empty input, mixed valid/invalid paths
  - handle_mouse_event: click, double-click, scroll, various button types

## [1.11.1] - 2026-01-31

### Changed
- **Refactoring**: Extracted duplicate archive sort logic into `ArchiveEntry::sort_entries()` method
  - Reduces code duplication between zip and tar.gz preview loading

### Added
- **Unit tests**: Added comprehensive tests for bookmark module
  - Mode change tests (BookmarkSet, BookmarkJump)
  - Bookmark set/get functionality tests
  - Invalid slot handling tests
  - Edge case tests (unset bookmark jump, no focus)

## [1.11.0] - 2026-01-31

### Changed
- **Trash support**: File deletion now moves files to system trash instead of permanent deletion
  - Uses the `trash` crate for cross-platform trash support
  - Safer file operations with ability to restore from trash
  - Updated confirmation dialog: "Move to Trash" instead of "Delete"

### Dependencies
- Added `trash` crate (v5) for trash operations

## [1.10.0] - 2026-01-31

### Added
- **tar.gz archive support**: Preview contents of tar.gz archives
  - Supported extensions: `.tar.gz`, `.tgz`
  - Shows file list with sizes and dates
  - Same sorting as zip archives (directories first)

### Dependencies
- Added `tar` crate (v0.4) for tar archive reading
- Added `flate2` crate (v1.0) for gzip decompression

## [1.9.2] - 2026-01-31

### Fixed
- **Scroll bounds checking**: Preview scroll now properly stops at the end of content
  - `PreviewScrollDown` (j) stops at last line instead of scrolling infinitely
  - `PreviewPageDown` (Ctrl+d) respects content boundaries
  - `PreviewToBottom` (G) now syncs with ViewMode scroll state
  - Applies to text, hex, and archive previews

### Added
- **Unit tests**: Added scroll bounds tests for all preview types

## [1.9.1] - 2026-01-31

### Fixed
- **Archive preview scroll**: Keyboard scroll (j/k, Ctrl+d/u, g/G) now works for archive previews
- **Hex preview scroll**: Keyboard scroll now works for hex/binary file previews
- **Mouse scroll**: Mouse wheel scroll now works for archive and hex previews

### Added
- **Unit tests**: Added comprehensive tests for `is_archive_file()` function
- **Unit tests**: Added tests for `ArchiveEntry` and `ArchivePreview` structures
- **Integration tests**: Added tests for archive preview functionality
  - Valid zip loading, corrupted zip error handling
  - Empty zip handling, scroll bounds verification
  - Entry sorting verification

## [1.9.0] - 2026-01-31

### Added
- **Archive preview**: Preview contents of zip archives
  - Supported formats: `.zip`, `.jar`, `.apk`, `.ipa`, `.xpi`, `.epub`
  - Shows file list with sizes and dates
  - Directories sorted first, then files alphabetically
- **Benchmark documentation**: Added `docs/BENCHMARKS.md`
  - Startup time: ~2.1ms (measured with hyperfine)

### Dependencies
- Added `zip` crate (v2.4) for archive reading

## [1.8.1] - 2026-01-31

### Documentation
- Added market analysis document (`docs/MARKET_ANALYSIS.md`)
- Updated ROADMAP with implemented features (v1.6.0-v1.8.0)

## [1.8.0] - 2026-01-31

### Added
- **Sort mode cycling**: Press `S` to cycle through sort modes
  - Name (default): Alphabetical order, case-insensitive
  - Size: Largest files first (descending)
  - Date: Newest files first (descending)
  - Directories are always sorted first within each mode
  - Current sort mode displayed in status bar when not default
- **Search match counter**: Display match count in status bar
  - Shows `3/12 matches` format during search
  - Updates when navigating between matches
  - Shows "No matches" when search finds nothing
- **Reverse search**: Press `N` (Shift+n) for previous match
  - `n` for next match (existing)
  - `N` for previous match (new)
  - Both wrap around at list boundaries

### Changed
- Help popup now shows `n/N` for search navigation and `S` for sort

## [1.7.2] - 2026-01-31

### Fixed
- **Revert git status flag**: Restore `-uall` to show all untracked files individually
  - `-unormal` in v1.7.1 hid individual files inside untracked directories

### Performance
- **Event polling**: Adjusted to 60ms (balanced between 50ms responsiveness and CPU savings)
- **Git polling**: Remains at 5 seconds (good balance)

## [1.7.1] - 2026-01-31

### Performance
- **Git polling interval**: 3 seconds → 5 seconds (40% fewer git operations)
- **Git status command**: Use `-unormal` instead of `-uall` (dramatically faster on large repos)
- **Event polling**: 50ms → 100ms timeout (50% fewer loop iterations when idle)

These changes significantly reduce CPU usage, especially when the application is idle or in large repositories.

## [1.7.0] - 2026-01-31

### Changed
- **Status bar redesign**: Show file info instead of position indicator
  - Display file size (e.g., `1.2 KB`, `4.5 MB`) for focused file
  - Display relative modification time (e.g., `2h ago`, `Yesterday`, `Jan 30`)
  - Format: `1.2 KB · 2h ago | Selected: 3`
  - Directories show `--` for size

### Removed
- Position indicator (`1/20`) - visual position in the list is sufficient

## [1.6.3] - 2026-01-31

### Added
- **Arrow key focus switching**: When side preview is visible, use `←`/`→` to switch focus
  - `←`: Focus tree panel (left)
  - `→`: Focus preview panel (right)
  - `l`/`h` keys continue to expand/collapse directories as before

### Documentation
- Updated keybindings documentation for new focus switching behavior

### Tests
- Added 6 unit tests for arrow key focus switching

## [1.6.2] - 2026-01-31

### Documentation
- Updated keybinding documentation to show toggle cancel behavior
- Added cancel key info for Search, Bookmarks, Filter, Fuzzy Finder

### Tests
- Added 15 unit tests for toggle behavior in key handlers

## [1.6.1] - 2026-01-31

### Improved
- **Toggle behavior for mode keys**: Same key that opens a mode can now close it
  - `m`: Press again to cancel bookmark set mode
  - `'`: Press again to cancel bookmark jump mode
  - `F`: Press again to cancel filter input mode
  - `/`: Press again to cancel search mode
  - More intuitive UX - no need to remember that Esc closes modes

## [1.6.0] - 2026-01-31

### Added
- **Bookmark feature**: Quick navigation with vim-style keybindings
  - `m1`-`m9`: Set bookmark at slot 1-9
  - `'1`-`'9`: Jump to bookmark at slot 1-9
  - 9 bookmark slots persist for current session
  - Status message shows bookmarked path when set
- **File filter feature**: Filter visible files by glob pattern
  - `F`: Open filter input / clear active filter
  - Supports `*` (any chars) and `?` (single char) wildcards
  - Examples: `*.rs`, `test*`, `*_test.py`
  - Directories always shown for navigation
  - Active filter displayed in status bar

### Changed
- Unified error message format across codebase
- Updated documentation (DESIGN.md, KEYBINDINGS.md, KEYBINDINGS_ja.md)

## [1.5.0] - 2026-01-31

### Changed
- **Major refactoring**: Split `handler/action.rs` (2,853 lines) into modular structure
  - `src/handler/action/mod.rs` - Type definitions and dispatch
  - `src/handler/action/navigation.rs` - Navigation actions (MoveUp, MoveDown, etc.)
  - `src/handler/action/tree_ops.rs` - Tree operations (Expand, Collapse, etc.)
  - `src/handler/action/selection.rs` - Selection and clipboard actions
  - `src/handler/action/file_ops.rs` - File operations (Paste, Delete, Rename, etc.)
  - `src/handler/action/search.rs` - Search and fuzzy finder actions
  - `src/handler/action/input.rs` - Input confirmation handling
  - `src/handler/action/display.rs` - Display, preview, and app control actions
- **Major refactoring**: Split `main.rs` (991 lines) into modular structure
  - `src/app/mod.rs` - Application module exports
  - `src/app/config.rs` - CLI configuration and argument parsing
  - `src/app/preview.rs` - Preview state management
  - `src/app/event_loop.rs` - Main event loop
  - `src/app/render.rs` - Rendering helpers
  - `src/main.rs` reduced to 81 lines (entry point only)

### Developer Experience
- Improved code organization with clear separation of concerns
- Each module has a single responsibility
- Easier to navigate and maintain the codebase

## [1.4.0] - 2026-01-30

### Added
- E2E test suite: 21 tests covering CLI behavior, error handling, and stdin mode
- Documentation: Complete keybindings reference in `docs/KEYBINDINGS.md`

### Changed
- README improvements: Shorter, more focused documentation with badges
- Added download count, CI status, and MSRV badges

### Developer Experience
- Added `assert_cmd` and `predicates` for CLI testing
- Test coverage now includes exit codes and error messages

## [1.3.0] - 2026-01-30

### Changed
- Smart file watching: Only watches expanded directories (much lighter)
- Git status polling: Updates every 3 seconds (shows changes in collapsed folders)

### Performance
- Significantly reduced resource usage on large projects
- Watching adapts dynamically to user's view

## [1.2.2] - 2026-01-30

### Fixed
- Exclude large directories from file watching: `.git`, `target`, `node_modules`, etc.
- Significantly improves performance by avoiding monitoring of build artifacts and dependencies

## [1.2.1] - 2026-01-30

### Fixed
- Fixed severe performance issue with file watcher: drain all pending events before refresh to prevent repeated expensive reloads

## [1.2.0] - 2026-01-30

### Added
- Real-time file watching: Automatically refreshes the tree and Git status when files change
  - Uses `notify` crate with 500ms debouncing for efficient updates
  - Eye icon () displayed in status bar when watching is active
  - Disabled in stdin mode (file watching not applicable)

## [1.1.0] - 2026-01-30

### Added
- `--stdin` option: Read paths from stdin for pipeline integration
  - Works with `find`, `fd`, `git diff --name-only`, etc.
  - Example: `find . -name "*.rs" | fv --stdin --pick`
- File operations (create, delete, rename, paste, refresh) are automatically disabled in stdin mode

## [1.0.1] - 2026-01-30

### Fixed
- Help display (`?` key) now shows a dedicated popup overlay instead of being truncated in the status bar

## [1.0.0] - 2026-01-30

### Changed
- Improved startup performance with lazy Git status detection
- Optimized tree loading with reduced stat() calls
- Unified exit codes across the application

### Added
- `EXIT_CODE_INVALID` (3) for invalid arguments/options
- Better error messages for invalid options and missing values

### Fixed
- `--format` now returns an error for invalid format values instead of silently using default
- `--on-select` now returns an error when command is missing
- Unknown options now return a proper error message

## [0.11.1] - 2026-01-30

### Changed
- Improved help message descriptions

## [0.11.0] - 2026-01-28

### Added
- Initial public release
- File tree navigation with keyboard and mouse
- Image preview (Kitty, iTerm2, Sixel protocols)
- Git status integration
- Fuzzy finder (Ctrl+P)
- File operations (create, delete, rename, copy, move)
- Pick mode for shell integration
- Nerd Fonts icons support
