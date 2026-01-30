# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
