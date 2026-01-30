# FileView Keybindings

Complete keybinding reference for FileView (fv).

[English](KEYBINDINGS.md) | [日本語](KEYBINDINGS_ja.md)

## Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Go to top |
| `G` | Go to bottom |

## Tree Operations

| Key | Action |
|-----|--------|
| `l` / `Tab` | Expand directory |
| `h` / `Backspace` | Collapse directory |
| `→` | Expand directory (or switch focus to preview when visible) |
| `←` | Collapse directory (or switch focus to tree when visible) |
| `Enter` | Toggle expand/collapse |
| `H` | Collapse all |
| `L` | Expand all (depth limit: 5) |

## Selection

| Key | Action |
|-----|--------|
| `Space` | Toggle mark |
| `Esc` | Clear all marks |

## File Operations

| Key | Action |
|-----|--------|
| `a` | Create new file |
| `A` | Create new directory |
| `r` | Rename |
| `D` / `Delete` | Delete (with confirmation) |
| `y` | Copy to clipboard |
| `d` | Cut to clipboard |
| `p` | Paste |

## Search

| Key | Action |
|-----|--------|
| `/` | Start search (press again to cancel) |
| `n` | Next search result |
| `N` | Previous search result |
| `Ctrl+P` | Open fuzzy finder |

Match count is displayed in status bar (e.g., `3/12 matches`).

## Sorting

| Key | Action |
|-----|--------|
| `S` | Cycle sort mode: Name → Size → Date → Name |

- **Name**: Alphabetical order (case-insensitive)
- **Size**: Largest files first
- **Date**: Newest files first
- Directories are always sorted first within each mode
- Current sort mode is shown in status bar when not default

### Fuzzy Finder

Press `Ctrl+P` to open the built-in fuzzy finder:

| Key | Action |
|-----|--------|
| `↑` / `Ctrl+K` | Move up in results |
| `↓` / `Ctrl+J` | Move down in results |
| `Enter` | Jump to selected file |
| `Esc` / `Ctrl+P` | Cancel |

- Type to filter files by name
- Results are sorted by match score
- Hidden files follow the current visibility setting

## Preview

| Key | Action |
|-----|--------|
| `P` | Toggle side preview panel |
| `o` | Open fullscreen preview |
| `Tab` | Toggle focus between tree and preview (when preview visible) |
| `←` / `→` | Switch focus to tree (left) / preview (right) |

### Side Preview Focus Mode

When the side preview panel is open, use `Tab` or arrow keys to switch focus:

| Focus | j/k/↑/↓ | g/G | b/f |
|-------|---------|-----|-----|
| Tree | Navigate files | Top/Bottom of list | - |
| Preview | Scroll content | Top/Bottom of preview | Page scroll |

- Click on a panel to switch focus
- Scroll wheel works on the focused panel
- `Esc` returns focus to tree
- Focused panel has cyan border highlight

## System Clipboard

| Key | Action |
|-----|--------|
| `c` | Copy path to system clipboard |
| `C` | Copy filename to system clipboard |

## Bookmarks

Press `m` followed by a digit (1-9) to set a bookmark, and `'` followed by a digit to jump:

| Key | Action |
|-----|--------|
| `m1`-`m9` | Set bookmark at slot 1-9 |
| `'1`-`'9` | Jump to bookmark at slot 1-9 |
| `m` / `'` | Cancel (press again without digit) |

- Bookmarks persist for the current session only
- A status message shows the bookmarked path when set
- Jumping to an unset bookmark shows "Bookmark N not set"

## File Filter

Press `F` to set or clear a file filter:

| Key | Action |
|-----|--------|
| `F` | Open filter input / clear filter |
| `Enter` | Apply filter |
| `Esc` / `F` | Cancel |

- Supports glob patterns: `*` (any chars), `?` (single char)
- Examples: `*.rs`, `test*`, `*_test.py`
- Directories are always shown for navigation
- Active filter is shown in status bar with filter icon
- Press `F` again when filter is active to clear it

## Other

| Key | Action |
|-----|--------|
| `.` | Toggle hidden files |
| `R` / `F5` | Refresh |
| `?` | Show help |
| `q` | Quit |
| `Q` | Quit and cd to current directory (with `--choosedir`) |

## Mouse Support

| Action | Effect |
|--------|--------|
| Click | Select item |
| Double-click | Expand/collapse directory or open preview |
| Scroll | Navigate list or scroll preview |
| Drag | Move files (experimental) |
