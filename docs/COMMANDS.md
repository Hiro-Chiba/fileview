# Custom Commands

FileView allows you to define custom shell commands and bind them to keys.

## Defining Commands

Commands are defined in `~/.config/fileview/config.toml`:

```toml
[commands]
open = "open $f"
edit = "nvim $f"
terminal = "cd $d && $SHELL"
compress = "zip -r archive.zip $S"
view = "less $f"
```

## Placeholders

| Placeholder | Description | Example |
|-------------|-------------|---------|
| `$f` | Full file path | `/home/user/documents/file.txt` |
| `$d` | Parent directory | `/home/user/documents` |
| `$n` | Filename with extension | `file.txt` |
| `$s` | Filename without extension (stem) | `file` |
| `$e` | File extension only | `txt` |
| `$S` | All selected files (quoted, space-separated) | `'/path/a.txt' '/path/b.txt'` |

## Binding Commands to Keys

In `~/.config/fileview/keymap.toml`:

```toml
[browse]
"o" = "command:open"
"e" = "command:edit"
"T" = "command:terminal"
```

The `command:` prefix tells FileView to execute the named command.

## Examples

### Open with Default Application

```toml
[commands]
open = "open $f"              # macOS
# open = "xdg-open $f"        # Linux
# open = "start $f"           # Windows
```

### Edit with Editor

```toml
[commands]
edit = "nvim $f"
# edit = "code $f"            # VS Code
# edit = "vim $f"             # Vim
```

### Open Terminal in Directory

```toml
[commands]
terminal = "cd $d && $SHELL"
```

### Archive Operations

```toml
[commands]
compress = "zip -r archive.zip $S"
extract = "unzip $f -d $d"
```

### File Operations

```toml
[commands]
chmod_exec = "chmod +x $f"
chown = "sudo chown $USER $S"
```

### View and Preview

```toml
[commands]
view = "less $f"
preview = "bat --style=plain $f"
hex = "xxd $f | less"
```

### Git Operations

```toml
[commands]
git_diff = "git diff $f"
git_log = "git log --oneline $f"
git_blame = "git blame $f | less"
```

### Media Operations

```toml
[commands]
play = "mpv $f"
convert = "ffmpeg -i $f ${s}_converted.mp4"
```

## Custom Preview

You can also define custom preview commands for specific file extensions:

```toml
[preview.custom]
md = "glow -s dark $f"
json = "jq -C . $f"
csv = "column -s, -t $f | head -50"
yaml = "yq -C . $f"
```

These are automatically used when previewing files with matching extensions.

## Tips

- Commands are executed via the shell (`sh -c` on Unix, `cmd /C` on Windows)
- Use `$S` for operations on multiple selected files
- Complex commands can be written as shell scripts
- Test commands in terminal first before binding them
