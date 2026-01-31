# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
