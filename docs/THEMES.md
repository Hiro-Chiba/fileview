# Theme Customization

FileView reads colors from `theme.toml` in the
[configuration directory](CONFIGURATION.md), usually
`~/.config/fileview/theme.toml`. Every key is optional.

## Theme File Structure

```toml
[colors]
selection = "darkgray"    # Background of the focused row
border = "default"        # Panel borders
border_active = "cyan"    # Border of the focused panel
warning = "yellow"        # Warning messages
info = "blue"             # Info messages
help_key_fg = "black"     # Key labels in the help popup
help_key_bg = "cyan"      # Background of those key labels

[file_colors]
directory = "blue"        # Directory names

[git_colors]
modified = "yellow"       # Modified file
staged = "green"          # Staged file
untracked = "green"       # Untracked file
deleted = "red"           # Deleted file
renamed = "cyan"          # Renamed file
conflict = "magenta"      # Conflict file
ignored = "darkgray"      # Ignored file
```

These are all the colors FileView draws with. Theme files written for versions
before 3.1.2 may also set `background`, `foreground`, `status_bg`, `status_fg`,
`error`, or file type colors such as `executable`. Those keys were never drawn and
are now ignored.

If the file cannot be parsed, FileView ignores it and uses the default colors.

## Color Formats

### Named Colors

- `black`, `red`, `green`, `yellow`, `blue`, `magenta` (also `purple`), `cyan`, `white`
- `gray` or `grey`, which is light gray, and `darkgray` or `darkgrey`
- `lightred`, `lightgreen`, `lightyellow`, `lightblue`, `lightmagenta`, `lightcyan`
- `default` or `reset`, which keep the terminal color

An unknown name also falls back to the terminal color.

### Hex Colors

```toml
selection = "#123"        # Short form, #RGB
directory = "#ff5733"     # Long form, #RRGGBB
```

### RGB Function

```toml
selection = "rgb(255, 87, 51)"
```

### 256-Color Palette

```toml
border = "color240"       # color followed by a number
ignored = "245"           # or just the number
```

## Example Theme

### Nord

```toml
[colors]
selection = "#434c5e"
border = "#4c566a"
border_active = "#88c0d0"

[file_colors]
directory = "#81a1c1"

[git_colors]
modified = "#ebcb8b"
staged = "#a3be8c"
untracked = "#a3be8c"
deleted = "#bf616a"
```

## Tips

- Leave `border` as `default` to inherit the terminal color
- Hex colors need a terminal with true color support
- The help key style can be switched with `FILEVIEW_HELP_KEY_STYLE=solid|outline|plain`
