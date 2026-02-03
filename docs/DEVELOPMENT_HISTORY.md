# FileView Development History

This document chronicles the development journey of FileView from initial release to the current version.

## Overview

FileView started as a simple terminal file browser and evolved into a feature-rich tool with image preview, Git integration, and Lua plugin support.

```
v0.11.0 ──► v1.0.0 ──► v1.14.0 ──► v1.18.0 ──► v1.22.0
  │           │           │           │           │
 MVP      Stable    Polished      Full       Plugins
                                Features
```

---

## Phase 1: Foundation (v0.11.0 - v1.0.0)

**January 2026**

### v0.11.0 - Initial Release

The first public release with core functionality:

- File tree navigation (Vim-style keybindings)
- Image preview (Kitty, iTerm2, Sixel protocols)
- Git status integration
- Fuzzy finder (Ctrl+P)
- File operations (create, delete, rename, copy, move)
- Pick mode for shell integration
- Nerd Fonts icons support

### v1.0.0 - Stable Release

Focus on stability and performance:

- Lazy Git status detection for faster startup
- Optimized tree loading with reduced stat() calls
- Unified exit codes (0: success, 1: cancelled, 2: error, 3: invalid args)
- Better error messages

---

## Phase 2: Core Features (v1.1.0 - v1.8.0)

**January 2026**

### v1.1.0 - Pipeline Integration

- `--stdin` option for reading paths from pipes
- Works with `find`, `fd`, `git diff --name-only`

### v1.2.0 - File Watching

- Real-time file watching with `notify` crate
- Auto-refresh on file changes
- 500ms debouncing for efficiency

### v1.3.0 - Smart Watching

- Only watch expanded directories
- Git status polling every 3 seconds

### v1.5.0 - Code Refactoring

- Split `handler/action.rs` (2,853 lines) into modular structure
- Split `main.rs` (991 lines) into `src/app/` module

### v1.6.0 - Bookmarks & Filters

- Bookmark feature (`m1`-`m9` to set, `'1`-`'9` to jump)
- File filter by glob pattern (`F` key)

### v1.7.0 - Status Bar Redesign

- Show file size and modification time
- Relative time display (e.g., "2h ago")

### v1.8.0 - Sort & Search

- Sort mode cycling (Name/Size/Date)
- Search match counter
- Reverse search (`N` key)

---

## Phase 3: Preview Enhancements (v1.9.0 - v1.14.0)

**January 2026**

### v1.9.0 - Archive Preview

- Zip archive contents preview
- Supported: `.zip`, `.jar`, `.apk`, `.ipa`, `.xpi`, `.epub`

### v1.10.0 - Tar.gz Support

- Added tar.gz archive preview
- Uses `tar` and `flate2` crates

### v1.11.0 - Trash Support

- File deletion now moves to system trash
- Cross-platform via `trash` crate

### v1.12.0 - Security

- Path traversal prevention in `--stdin` mode
- Git command PATH hardening
- TOCTOU mitigation in file operations

### v1.13.0 - PDF Preview

- PDF rendering via poppler-utils
- Page navigation (`[` / `]` keys)

### v1.14.0 - Syntax Highlighting

- 100+ languages supported via `syntect`
- Same engine as bat/delta

---

## Phase 4: Customization (v1.15.0 - v1.18.0)

**January - February 2026**

### v1.15.0 - Git Operations

- Async image loading (background threads)
- Git stage/unstage (`s` / `u` keys)
- Git diff preview (color-coded)
- Bulk rename with patterns

### v1.16.0 - Configuration System

- Config file: `~/.config/fileview/config.toml`
- Keymap customization: `keymap.toml`
- Theme system: `theme.toml`

### v1.17.0 - Custom Commands

- Define shell commands in config
- Custom preview via external scripts

### v1.18.0 - Full Features

- Video preview (ffmpeg integration)
- Tab support (Ctrl+T, Ctrl+W)
- Event hooks system
- Shell integration (`--choosedir`, `--selection-path`)
- Visual select mode (`V` key)
- Batch operations

---

## Phase 5: Plugin System (v1.19.0 - v1.22.0)

**February 2026**

### v1.19.0 - Lua Runtime (Phase 12a)

- Lua 5.4 integration via `mlua` crate
- Read-only API: `fv.current_file()`, `fv.current_dir()`, etc.
- Plugin location: `~/.config/fileview/plugins/init.lua`

### v1.20.0 - Action API (Phase 12b)

- Action functions: `fv.navigate()`, `fv.select()`, `fv.refresh()`
- Actions queued and processed by event loop

### v1.21.0 - Registration API (Phase 12c)

- `fv.register_command()`: Custom commands
- `fv.on()`: Event handlers
- `fv.register_previewer()`: Custom previewers

### v1.22.0 - Integration (Phase 12d)

- Automatic plugin loading at startup
- Event firing: `start`, `file_selected`, `directory_changed`, etc.
- Full plugin system integration

---

## Statistics

### Code Growth

| Version | Approx. Lines |
|---------|---------------|
| v0.11.0 | ~5,000 |
| v1.0.0  | ~6,000 |
| v1.14.0 | ~10,000 |
| v1.18.0 | ~15,000 |
| v1.22.0 | ~18,000 |

### Test Coverage

| Version | Tests |
|---------|-------|
| v1.0.0  | ~100 |
| v1.14.0 | ~340 |
| v1.18.0 | ~430 |
| v1.22.0 | ~450 |

### Dependencies Added

| Version | Crate | Purpose |
|---------|-------|---------|
| v1.2.0 | `notify` | File watching |
| v1.9.0 | `zip` | Archive preview |
| v1.10.0 | `tar`, `flate2` | Tar.gz support |
| v1.11.0 | `trash` | Trash operations |
| v1.13.0 | `tempfile` | PDF temp files |
| v1.14.0 | `syntect` | Syntax highlighting |
| v1.16.0 | `toml`, `dirs`, `serde` | Configuration |
| v1.19.0 | `mlua` | Lua plugins |

---

## Design Principles

Throughout development, these principles guided decisions:

1. **Simple** - Focus on essential features
2. **Fast** - Startup < 50ms, smooth operation
3. **Zero-config** - Works immediately after install
4. **Batteries included** - Image preview, Git, syntax highlighting built-in

---

## What's Next

See [PLUGINS.md](PLUGINS.md) for the Lua plugin system documentation.

For detailed changes in each version, see [CHANGELOG.md](../CHANGELOG.md).
