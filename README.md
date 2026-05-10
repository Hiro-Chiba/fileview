# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![Downloads](https://img.shields.io/crates/d/fileview.svg)](https://crates.io/crates/fileview)
[![CI](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml/badge.svg)](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.90-blue.svg)](https://www.rust-lang.org)

> Terminal file browser with image preview, written in Rust.

English | [日本語](README_ja.md)

## Demo

<p align="center">
  <img src="assets/demo.gif" alt="FileView Demo" width="80%">
</p>

Wanted a yazi-like file manager for day-to-day terminal use that also talks to Claude Code. So I wrote this.

## Features

- Auto image preview (Kitty, iTerm2, Sixel, Halfblocks)
- 2.2ms startup, ~8MB memory ([benchmarks](docs/BENCHMARKS.md))
- Git status, syntax highlighting, PDF preview, fuzzy finder
- Vim keybindings, mouse support, Lua plugins
- Live reflection of `fv --mcp-server` activity in the TUI ([details](docs/CLAUDE_CODE.md))

## Quick Start

```bash
cargo install fileview
fv
```

## Install Options

Chafa image support: `cargo install fileview --features chafa`<br>
Speed-optimized build: `cargo install fileview --profile release-fast`<br>
Slim build (drops the `arboard` clipboard and `mlua` Lua plugin
dependencies): `cargo install fileview --no-default-features`<br>
Pick individual features: `cargo install fileview --no-default-features --features clipboard,lua`

## Image Preview

Your terminal is auto-detected:

| Terminal | Protocol |
|----------|----------|
| Kitty / Ghostty / Konsole | Kitty Graphics |
| iTerm2 / WezTerm / Warp | iTerm2 Inline |
| Foot / Windows Terminal | Sixel |
| VS Code / Alacritty | Halfblocks |

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate up/down |
| `h/l` | Collapse/expand |
| `g/G` | Top/bottom |
| `Space` | Toggle mark |
| `/` | Search |
| `Ctrl+P` | Fuzzy finder |
| `P` | Preview panel |
| `q` | Quit |

See [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md) for the full list.

## Claude Code Integration

FileView includes an MCP server for Claude Code (`fv --mcp-server`).

```json
{
  "mcpServers": {
    "fileview": {
      "command": "fv",
      "args": ["--mcp-server"]
    }
  }
}
```

Details: [docs/CLAUDE_CODE.md](docs/CLAUDE_CODE.md)

## Docs

- [Keybindings](docs/KEYBINDINGS.md)
- [Claude Code / MCP](docs/CLAUDE_CODE.md)
- [Lua Plugins](docs/PLUGINS.md)


## License

MIT
