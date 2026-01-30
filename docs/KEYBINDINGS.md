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
| `l` / `→` / `Tab` | Expand directory |
| `h` / `←` / `Backspace` | Collapse directory |
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
| `/` | Start search |
| `n` | Next search result |
| `Ctrl+P` | Open fuzzy finder |

### Fuzzy Finder

Press `Ctrl+P` to open the built-in fuzzy finder:

| Key | Action |
|-----|--------|
| `↑` / `Ctrl+K` | Move up in results |
| `↓` / `Ctrl+J` | Move down in results |
| `Enter` | Jump to selected file |
| `Esc` | Cancel |

- Type to filter files by name
- Results are sorted by match score
- Hidden files follow the current visibility setting

## Preview

| Key | Action |
|-----|--------|
| `P` | Toggle side preview panel |
| `o` | Open fullscreen preview |
| `Tab` | Toggle focus between tree and preview (when preview visible) |

### Side Preview Focus Mode

When the side preview panel is open, press `Tab` to switch focus:

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
