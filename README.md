# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A minimal file tree browser for terminal emulators with vim-like keybindings and image preview.

English | [日本語](README_ja.md)

## Features

- Fast file tree navigation with vim-like keybindings
- **Git integration** - color-coded file status and branch display
- Multi-select support for batch operations
- **Preview panel** with support for:
  - Text files (with line numbers)
  - Images (Kitty/iTerm2/Sixel protocols with auto-detection)
  - Directories (file count, size statistics)
  - Binary files (hex dump view)
- Copy/cut/paste with internal clipboard
- System clipboard integration (path/filename copy)
- Pick mode for external tool integration
- Callback execution on file selection
- Hidden files toggle
- Mouse support (click, double-click, scroll, drag-and-drop)
- **Nerd Fonts icons** (enabled by default, disable with `--no-icons`)

## Git Status Colors

When inside a git repository, files are color-coded by their status:

| Color | Status |
|-------|--------|
| Yellow | Modified |
| Green | Added / Untracked |
| Red | Deleted |
| Cyan | Renamed |
| Gray | Ignored |
| Magenta | Conflict |

The current branch name is displayed in the status bar.

## Image Preview

FileView automatically detects your terminal and selects the optimal image protocol:

| Terminal | Protocol | Quality |
|----------|----------|---------|
| Kitty | Kitty Graphics | Highest |
| Ghostty | Kitty Graphics | Highest |
| Konsole | Kitty Graphics | Highest |
| iTerm2 | iTerm2 Inline | Highest |
| WezTerm | iTerm2 Inline | Highest |
| Warp | iTerm2 Inline | Highest |
| Foot | Sixel | Good |
| Windows Terminal | Sixel | Good |
| VS Code | Halfblocks | Basic |
| Alacritty | Halfblocks | Basic |
| Other | Auto-detect | Varies |

Override with `FILEVIEW_IMAGE_PROTOCOL` environment variable (see below).

## Installation

### From crates.io (Recommended)

```bash
cargo install fileview
```

### With Chafa support (optional)

For better image quality on terminals without native image protocol support:

```bash
# Install libchafa first
# macOS:
brew install chafa

# Ubuntu/Debian:
sudo apt install libchafa-dev

# Then install with chafa feature
cargo install fileview --features chafa
```

### From source

```bash
git clone https://github.com/Hiro-Chiba/fileview.git
cd fileview
cargo install --path .

# Or with chafa:
cargo install --path . --features chafa
```

### Requirements

- Rust 1.70+
- A terminal with true color support (recommended: Ghostty, iTerm2, Alacritty)

## Usage

```bash
# Open current directory
fv

# Open specific directory
fv /path/to/directory

# Pick mode (output selected path to stdout)
fv --pick

# Pick mode with JSON output
fv --pick --format json

# Execute command on selection
fv --on-select "code {path}"
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Go to top |
| `G` | Go to bottom |

### Tree Operations

| Key | Action |
|-----|--------|
| `l` / `→` / `Tab` | Expand directory |
| `h` / `←` / `Backspace` | Collapse directory |
| `Enter` | Toggle expand/collapse |
| `H` | Collapse all |
| `L` | Expand all (depth limit: 5) |

### Selection

| Key | Action |
|-----|--------|
| `Space` | Toggle mark |
| `Esc` | Clear all marks |

### File Operations

| Key | Action |
|-----|--------|
| `a` | Create new file |
| `A` | Create new directory |
| `r` | Rename |
| `D` / `Delete` | Delete (with confirmation) |
| `y` | Copy to clipboard |
| `d` | Cut to clipboard |
| `p` | Paste |

### Search

| Key | Action |
|-----|--------|
| `/` | Start search |
| `n` | Next search result |
| `Ctrl+P` | Open fuzzy finder |

### Fuzzy Finder

Press `Ctrl+P` to open the built-in fuzzy finder for quick file navigation:

| Key | Action |
|-----|--------|
| `↑` / `Ctrl+K` | Move up in results |
| `↓` / `Ctrl+J` | Move down in results |
| `Enter` | Jump to selected file |
| `Esc` | Cancel |

- Type to filter files by name
- Results are sorted by match score
- Hidden files follow the current visibility setting

### Preview

| Key | Action |
|-----|--------|
| `P` | Toggle side preview panel |
| `o` | Open fullscreen preview |
| `Tab` | Toggle focus between tree and preview (when preview visible) |

#### Side Preview Focus Mode

When the side preview panel is open, press `Tab` to switch focus:

| Focus | j/k/↑/↓ | g/G | b/f |
|-------|---------|-----|-----|
| Tree | Navigate files | Top/Bottom of list | - |
| Preview | Scroll content | Top/Bottom of preview | Page scroll |

- Click on a panel to switch focus
- Scroll wheel works on the focused panel
- `Esc` returns focus to tree
- Focused panel has cyan border highlight

### Other

| Key | Action |
|-----|--------|
| `.` | Toggle hidden files |
| `R` / `F5` | Refresh |
| `c` | Copy path to system clipboard |
| `C` | Copy filename to system clipboard |
| `?` | Show help |
| `q` | Quit |
| `Q` | Quit and cd to current directory (with `--choosedir`) |

## CLI Options

| Option | Description |
|--------|-------------|
| `-p`, `--pick` | Pick mode: output selected path(s) to stdout |
| `-f`, `--format FMT` | Output format: `lines` (default), `null`, `json` |
| `--on-select CMD` | Run command when file is selected |
| `--choosedir` | Output directory path on exit (for shell cd integration) |
| `-i`, `--icons` | Enable Nerd Fonts icons (default) |
| `--no-icons` | Disable icons |
| `-h`, `--help` | Show help |
| `-V`, `--version` | Show version |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `FILEVIEW_ICONS=0` | Disable icons |
| `FILEVIEW_IMAGE_PROTOCOL` | Force image protocol: `auto`, `halfblocks`, `chafa`, `sixel`, `kitty`, `iterm2` |

### Placeholders for `--on-select`

| Placeholder | Description |
|-------------|-------------|
| `{path}` | Full path |
| `{dir}` | Parent directory |
| `{name}` | Filename with extension |
| `{stem}` | Filename without extension |
| `{ext}` | Extension only |

### Shell Integration

Navigate directories with fileview and cd to the selected location:

```bash
# Add to your .bashrc or .zshrc
fvcd() {
  local dir
  dir=$(fv --choosedir "$@")
  if [ -n "$dir" ] && [ -d "$dir" ]; then
    cd "$dir"
  fi
}
```

Usage:
- Run `fvcd` to open fileview
- Navigate to your target directory
- Press `Q` to quit and cd there
- Press `q` to quit without changing directory

### Examples

```bash
# Use as file picker for another tool
selected=$(fv --pick)
echo "Selected: $selected"

# Open selected file in editor
fv --on-select "vim {path}"

# Copy selected file path to clipboard (macOS)
fv --on-select "echo {path} | pbcopy"

# Multiple file selection with JSON output
fv --pick --format json
```

## License

MIT
