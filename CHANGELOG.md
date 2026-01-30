# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
