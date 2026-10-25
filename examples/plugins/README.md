# FileView Plugin Example

`init.lua` shows the event handlers FileView supports. Copy it to
`~/.config/fileview/plugins/init.lua`, or run `fv plugin init` and paste the parts
you want. Only `init.lua` is loaded.

Check a plugin without starting the TUI:

```sh
fv plugin test examples/plugins/init.lua
```

See [docs/PLUGINS.md](../../docs/PLUGINS.md) for the full API reference.
