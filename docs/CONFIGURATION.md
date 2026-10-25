# Configuration Guide

FileView works without any configuration. Optional settings live in TOML files in
`~/.config/fileview/`, or in `$XDG_CONFIG_HOME/fileview/` when that variable is
set.

Versions before 3.1.2 read these files from the platform folder instead, which is
`~/Library/Application Support/fileview/` on macOS and `%APPDATA%\fileview\` on
Windows. A file there is still read when the same file is missing from
`~/.config/fileview/`, so you do not need to move anything.

## Configuration Files

| File | Description |
|------|-------------|
| `config.toml` | Main settings |
| `theme.toml` | Colors, see the [theme guide](THEMES.md) |
| `plugins/init.lua` | Lua plugin, see the [plugin guide](PLUGINS.md) |

If a file cannot be parsed, FileView ignores the whole file and uses the defaults,
without an error message. Keys that FileView does not know are ignored, so a file
written for an older version still loads.

## Main Configuration (`config.toml`)

### General Settings

```toml
[general]
show_hidden = false       # Show hidden files on start
enable_icons = true       # Show Nerd Font icons next to file names
mouse_enabled = true      # Capture the mouse for clicks, scrolling, and drops
```

### Custom Previews

```toml
[preview.custom]
csv = "column -s, -t $f | head -50"
log = "tail -n 200 $f"
```

Each entry maps a file extension to a shell command whose output replaces the
built-in preview. `$f` is replaced with the shell-escaped file path. No other
placeholder is supported here. The extension match is case-sensitive, so `csv`
does not match `DATA.CSV`. The output is shown as plain text, so color codes from
tools such as `jq -C` or `glow` are not drawn as colors.

### Performance Settings

```toml
[performance]
git_poll_interval_secs = 5   # How often Git status is refreshed, in seconds
```

### Token Budget

```toml
[budget]
default_model = "sonnet"     # sonnet, sonnet-1m, opus, haiku, or a full name
```

This picks the model whose context window the token budget bar measures against.
`Alt+M` cycles through the models while FileView is running.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `FILEVIEW_ICONS=0` | Hide icons |
| `FILEVIEW_IMAGE_PROTOCOL` | Image protocol: `auto`, `halfblocks`, `chafa`, `sixel`, `kitty`, or `iterm2` |
| `FILEVIEW_HELP_KEY_STYLE` | Help key style: `solid`, `outline`, or `plain` |

## CLI Arguments

```
--hidden, -a     Show hidden files
--no-hidden      Hide hidden files
--icons, -i      Show icons
--no-icons       Hide icons
```

## Priority

For hidden files, an argument wins over `config.toml`, which wins over the
default. For icons, an argument wins over `FILEVIEW_ICONS`, which wins over
`config.toml`, which wins over the default.

## Example Configuration

See [examples/config.toml](../examples/config.toml) for a complete example.
