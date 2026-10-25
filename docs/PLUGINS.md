# Lua Plugin System

FileView can run a Lua plugin that reacts to events, shows notifications, and
moves the focus or selection.

## Quick Start

Create the plugin template, then edit it:

```sh
fv plugin init
```

This writes `plugins/init.lua` in the [configuration directory](CONFIGURATION.md),
usually `~/.config/fileview/plugins/init.lua`:

```lua
fv.notify("Hello from plugin!")
```

Restart FileView to see the notification. `fv plugin test [PATH]` loads a plugin
file and prints its notifications without starting the TUI.

## Plugin Location

Only `plugins/init.lua` is loaded, once at startup. Other `.lua` files in the
folder run only if `init.lua` loads them with `require` or `dofile`.

## API Reference

### Read-only Functions

| Function | Returns | Description |
|----------|---------|-------------|
| `fv.current_file()` | `string` or `nil` | Currently focused file path |
| `fv.current_dir()` | `string` | Root directory of the tree |
| `fv.selected_files()` | `table` | Array of marked file paths |
| `fv.version()` | `string` | FileView version, for example `"3.1.2"` |
| `fv.is_dir(path)` | `boolean` | Whether the path is a directory |
| `fv.file_exists(path)` | `boolean` | Whether the path exists |

### Action Functions

| Function | Description |
|----------|-------------|
| `fv.notify(message)` | Show a message in the status bar |
| `fv.navigate(path)` | Make a directory the tree root |
| `fv.select(path)` | Mark a file |
| `fv.deselect(path)` | Unmark a file |
| `fv.clear_selection()` | Clear all marks |
| `fv.refresh()` | Reload the tree |
| `fv.set_clipboard(text)` | Set the system clipboard |
| `fv.focus(path)` | Reveal a file in the tree and focus it |

### Event Handlers

| Function | Description |
|----------|-------------|
| `fv.on(event, fn)` | Run `fn` when `event` fires. Several handlers can share an event |

`fv.register_command` and `fv.register_previewer` still exist so that older
plugins keep loading, but FileView never calls what they register. There is no
command prompt to run commands from, and previews do not consult plugins.

## Events

| Event | Argument | Fires when |
|-------|----------|------------|
| `start` | none | Plugins have loaded |
| `file_selected` | `path` | The focused file changes |
| `directory_changed` | `path` | The tree root changes |
| `selection_changed` | none | The number of marked files changes |
| `before_quit` | none | FileView is about to exit |

## Examples

### Warn on Secret Files

```lua
fv.on("file_selected", function(path)
    if path and path:match("%.env$") then
        fv.notify("Warning: environment file")
    end
end)
```

### Start in the Home Directory

```lua
fv.on("start", function()
    local home = os.getenv("HOME")
    if home then
        fv.navigate(home)
    end
end)
```

### Report the Marked Files

```lua
fv.on("selection_changed", function()
    fv.notify(#fv.selected_files() .. " file(s) marked")
end)
```

A complete example is in [examples/plugins/init.lua](../examples/plugins/init.lua).

## Errors

If `init.lua` fails to load, the status bar shows the error. Errors raised inside
an event handler are discarded without a message, so test handlers with
`fv plugin test`.

## Security

Plugins are **not** sandboxed. They run as trusted user code in Lua 5.4 with the
standard libraries except `debug`, and loading C modules is disabled.
`os.execute`, `io.popen`, `io.open`, and `require` for Lua files are available,
so a plugin can run any command, read and write any file your account can access,
and reach the network through those commands. This is equivalent to running a
shell script you wrote yourself.

Consequences to be aware of:

- `plugins/init.lua` runs automatically on every startup. Anything able to write
  that file gains code execution the next time you launch FileView, so treat the
  plugin directory like any other executable on your `PATH`.
- Only install plugin code you have read and trust.
- When a plugin builds a shell command from a file path, quote it. For example
  `os.execute("git add " .. file)` is unsafe for a file named `; rm -rf ~`.
  Escape the path or avoid the shell.

To run without any plugin code, build with the `lua` feature disabled:

```sh
cargo install fileview --locked --no-default-features --features ai,archive,clipboard,svg
```
