# Theme Customization

FileView supports custom color themes via `~/.config/fileview/theme.toml`.

## Theme File Structure

```toml
[colors]
background = "default"    # Main background
foreground = "white"      # Default text color
selection = "blue"        # Selection highlight
border = "gray"           # Border color
help_key_fg = "black"     # Help popup key text color
help_key_bg = "cyan"      # Help popup key background color

[file_colors]
directory = "blue"        # Directory color
executable = "green"      # Executable file color
symlink = "cyan"          # Symbolic link color
archive = "red"           # Archive file color (.zip, .tar, etc.)
image = "magenta"         # Image file color
media = "magenta"         # Media file color

[git_colors]
modified = "yellow"       # Modified file
staged = "green"          # Staged file
untracked = "red"         # Untracked file
conflict = "red"          # Conflict file
```

## Color Formats

FileView supports multiple color formats:

### Named Colors

```toml
foreground = "white"
directory = "blue"
modified = "yellow"
```

Available named colors:
- `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`
- `gray`, `grey` (dark gray)
- `lightred`, `lightgreen`, `lightyellow`, `lightblue`, `lightmagenta`, `lightcyan`
- `darkgray`, `darkgrey`
- `default` (terminal default)
- `reset` (reset to default)

### Hex Colors

```toml
# Short form (#RGB)
background = "#123"

# Long form (#RRGGBB)
foreground = "#ff5733"
```

### RGB Function

```toml
selection = "rgb(255, 87, 51)"
```

### 256-Color Palette

```toml
# Using color<N> or colorN syntax
border = "color240"
accent = "color33"

# Or just the number
dim = "245"
```

## Example Themes

### Dark Theme (Default)

```toml
[colors]
background = "default"
foreground = "white"
selection = "blue"
border = "gray"

[file_colors]
directory = "blue"
executable = "green"
symlink = "cyan"

[git_colors]
modified = "yellow"
staged = "green"
untracked = "red"
```

### Light Theme

```toml
[colors]
background = "#ffffff"
foreground = "#1a1a1a"
selection = "#0066cc"
border = "#cccccc"

[file_colors]
directory = "#0066cc"
executable = "#008800"
symlink = "#008888"

[git_colors]
modified = "#cc6600"
staged = "#008800"
untracked = "#cc0000"
```

### Nord Theme

```toml
[colors]
background = "#2e3440"
foreground = "#eceff4"
selection = "#5e81ac"
border = "#4c566a"

[file_colors]
directory = "#81a1c1"
executable = "#a3be8c"
symlink = "#88c0d0"

[git_colors]
modified = "#ebcb8b"
staged = "#a3be8c"
untracked = "#bf616a"
```

## Tips

- Use `default` for background to inherit terminal background
- Use 256-color palette (`color0`-`color255`) for precise control
- Hex colors provide the most flexibility
- Test your theme in different terminal emulators for consistency
- Help key highlight style can be switched with `FILEVIEW_HELP_KEY_STYLE=solid|outline|plain`
