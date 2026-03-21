# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![Downloads](https://img.shields.io/crates/d/fileview.svg)](https://crates.io/crates/fileview)
[![CI](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml/badge.svg)](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.90-blue.svg)](https://www.rust-lang.org)

> Zero-config terminal file browser with image preview, built in Rust.

English | [日本語](README_ja.md)

## Demo

<p align="center">
  <img src="assets/demo.gif" alt="FileView Demo" width="80%">
</p>

## Features

- Auto image preview (Kitty, iTerm2, Sixel, Halfblocks)
- 2.2ms startup, ~8MB memory
- Git status, syntax highlighting, PDF preview, fuzzy finder
- Vim keybindings, mouse support, Lua plugins

## Quick Start

```bash
cargo install fileview
fv
```

## Install Options

Chafa image support: `cargo install fileview --features chafa`<br>
Speed-optimized build: `cargo install fileview --profile release-fast`

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
- [Comparison with other file managers](docs/COMPARISON.md)

## License

MIT
