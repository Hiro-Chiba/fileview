# Configuration Guide

FileView supports configuration through TOML files located in `~/.config/fileview/`.

## Configuration Files

| File | Description |
|------|-------------|
| `config.toml` | Main settings (general, preview, UI, performance) |
| `keymap.toml` | Custom key bindings |
| `theme.toml` | Color theme customization |

## Main Configuration (`config.toml`)

### General Settings

```toml
[general]
show_hidden = false       # Show hidden files by default
enable_icons = true       # Enable Nerd Font icons
mouse_enabled = true      # Enable mouse support
```

### Preview Settings

```toml
[preview]
hex_max_bytes = 4096         # Maximum bytes for hex preview
max_archive_entries = 500    # Maximum entries for archive preview
image_protocol = "auto"      # Image protocol: auto, sixel, kitty, iterm2, halfblocks, chafa

# Custom preview commands (extension -> command)
[preview.custom]
md = "glow -s dark $f"       # Markdown with glow
json = "jq -C . $f"          # JSON with jq
csv = "column -s, -t $f | head -50"
```

### Performance Settings

```toml
[performance]
git_poll_interval_secs = 5   # Git status polling interval
```

### UI Settings

```toml
[ui]
show_size = true                    # Show file sizes in tree view
show_permissions = false            # Show file permissions
date_format = "%Y-%m-%d %H:%M"      # Date format (strftime-style)
```

### Custom Commands

```toml
[commands]
# Define commands that can be bound to keys
# Placeholders: $f (full path), $d (directory), $n (filename), $s (stem), $e (extension), $S (selected files)
open = "open $f"
edit = "nvim $f"
terminal = "cd $d && $SHELL"
compress = "zip -r archive.zip $S"
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `FILEVIEW_ICONS=0` | Disable icons |
| `FILEVIEW_IMAGE_PROTOCOL` | Force image protocol |
| `FILEVIEW_HELP_KEY_STYLE` | Help key style: `solid`, `outline`, `plain` |

## CLI Arguments

CLI arguments override config file settings:

```
--hidden, -a     Show hidden files
--no-hidden      Hide hidden files
--icons, -i      Enable icons
--no-icons       Disable icons
```

## Configuration Priority

1. CLI arguments (highest priority)
2. Environment variables
3. Configuration file (`~/.config/fileview/config.toml`)
4. Default values (lowest priority)

## Example Configuration

See [examples/config.toml](../examples/config.toml) for a complete example.
