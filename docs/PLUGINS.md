# Lua Plugin System

FileView supports Lua plugins for extending functionality.

## Quick Start

Create a plugin file at `~/.config/fileview/plugins/init.lua`:

```lua
fv.notify("Hello from plugin!")
```

Restart FileView to see the notification.

## Plugin Location

```
~/.config/fileview/plugins/init.lua
```

This file is automatically loaded at startup.

## API Reference

### Read-only Functions

| Function | Returns | Description |
|----------|---------|-------------|
| `fv.current_file()` | `string` or `nil` | Currently focused file path |
| `fv.current_dir()` | `string` | Current directory path |
| `fv.selected_files()` | `table` | Array of selected file paths |
| `fv.version()` | `string` | FileView version (e.g., "1.22.0") |
| `fv.is_dir(path)` | `boolean` | Check if path is a directory |
| `fv.file_exists(path)` | `boolean` | Check if path exists |

### Action Functions

| Function | Description |
|----------|-------------|
| `fv.notify(message)` | Display notification in status bar |
| `fv.navigate(path)` | Navigate to a directory |
| `fv.select(path)` | Add file to selection |
| `fv.deselect(path)` | Remove file from selection |
| `fv.clear_selection()` | Clear all selections |
| `fv.refresh()` | Reload tree view |
| `fv.set_clipboard(text)` | Set clipboard content |
| `fv.focus(path)` | Focus on specific file (reveal and select) |

### Registration Functions

| Function | Description |
|----------|-------------|
| `fv.register_command(name, fn)` | Register a custom command |
| `fv.on(event, fn)` | Register an event handler |
| `fv.register_previewer(pattern, fn)` | Register a custom previewer |

## Events

| Event | Argument | Description |
|-------|----------|-------------|
| `start` | none | After plugins load |
| `file_selected` | `path` | When focused file changes |
| `directory_changed` | `path` | When navigating to new directory |
| `selection_changed` | none | When multi-selection changes |
| `before_quit` | none | Before application exits |

## Examples

### Copy File Path to Clipboard

```lua
fv.register_command("copy-path", function()
    local file = fv.current_file()
    if file then
        fv.set_clipboard(file)
        fv.notify("Copied: " .. file)
    else
        fv.notify("No file selected")
    end
end)
```

### Warn on Secret Files

```lua
fv.on("file_selected", function(path)
    if path and path:match("%.env$") then
        fv.notify("Warning: Environment file!")
    end
end)
```

### Navigate to Home on Startup

```lua
fv.on("start", function()
    local home = os.getenv("HOME")
    if home then
        fv.navigate(home)
    end
end)
```

### Custom CSV Previewer

```lua
fv.register_previewer("*.csv", function(path)
    local file = io.open(path, "r")
    if not file then
        return "Cannot read file"
    end

    local lines = {}
    local count = 0
    for line in file:lines() do
        table.insert(lines, line)
        count = count + 1
        if count >= 50 then break end
    end
    file:close()

    return table.concat(lines, "\n")
end)
```

### Select All Files with Extension

```lua
fv.register_command("select-rs", function()
    local dir = fv.current_dir()
    -- Note: This is a simple example. Real implementation
    -- would need to iterate through directory contents.
    fv.notify("Select *.rs files in " .. dir)
end)
```

## Glob Patterns for Previewers

The `register_previewer` function supports glob patterns:

| Pattern | Matches |
|---------|---------|
| `*.txt` | `file.txt`, `readme.txt` |
| `*.??` | `file.rs`, `test.py` |
| `test*` | `test.txt`, `testing.md` |
| `*test*` | `my_test_file`, `testing` |

## Notes

- Plugins run in a sandboxed Lua 5.4 environment
- Errors in plugins are caught and displayed as notifications
- Multiple handlers can be registered for the same event
- Previewers return strings that are displayed in the preview panel
