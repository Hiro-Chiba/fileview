# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![Downloads](https://img.shields.io/crates/d/fileview.svg)](https://crates.io/crates/fileview)
[![CI](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml/badge.svg)](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.90-blue.svg)](https://www.rust-lang.org)

> Browse files with image previews, Git status, and search in your terminal. No configuration needed.

<p align="center">
  <img src="assets/demo.gif" alt="FileView terminal file browser demo" width="80%">
</p>

## Quick Start

[Download a prebuilt binary](https://github.com/Hiro-Chiba/fileview/releases/latest)
for your OS. Rust is not required. Choose the archive with the matching filename
ending below, then extract it and run the included executable from a terminal.

| System | Archive filename ending |
| --- | --- |
| macOS, Apple Silicon | `aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `x86_64-apple-darwin.tar.gz` |
| Linux, x86-64 (GNU) | `x86_64-unknown-linux-gnu.tar.gz` |
| Windows, x86-64 | `x86_64-pc-windows-msvc.zip` |

### macOS and Linux

Save your downloaded archive as `fileview.tar.gz`. In the folder containing it, run:

```sh
tar -xzf fileview.tar.gz
./fv
```

### Windows (PowerShell)

Save your downloaded archive as `fileview.zip`. In the folder containing it, run:

```powershell
Expand-Archive -Path .\fileview.zip -DestinationPath .\fileview
.\fileview\fv.exe
```

Use `j/k` to move, `/` to search, `P` to toggle the preview, and `q` to quit.
Git status requires Git to be installed. Image previews adapt to your terminal.

### Install with Cargo

If you already have Rust 1.90 or newer:

```sh
cargo install fileview --locked
fv
```

## Features

- Image previews with automatic terminal detection (Kitty, iTerm2, Sixel, Halfblocks)
- Git status, syntax highlighting, search, and fuzzy finder
- PDF previews with Poppler's `pdftoppm` installed
- Vim keybindings, mouse support, Lua plugins
- Live reflection of `fv --mcp-server` activity in the TUI ([details](docs/CLAUDE_CODE.md))

See the [performance comparison](docs/BENCHMARKS.md) for local measurements and
their test conditions.

## Install Options

Chafa image support: `cargo install fileview --features chafa`<br>
Speed-optimized build: `cargo install fileview --profile release-fast`<br>
Slim build (drops `arboard` clipboard, `mlua` Lua plugin,
`zip` / `tar` / `flate2` archive, and `tiktoken-rs` /  `petgraph`
AI helper dependencies): `cargo install fileview --no-default-features`<br>
Pick individual features: `cargo install fileview --no-default-features --features ai,clipboard,lua,archive`

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

## One-shot helpers for AI workflows

A few non-interactive flags for use from scripts and AI agents:

```bash
fv --tokens README.md            # cl100k_base token estimate (one integer to stdout)
fv --snapshot-create base        # capture working tree manifest to .fileview/snapshots/
fv --snapshot-diff base          # + added / - removed / M modified since snapshot
fv --watch path/to/file          # block until the file changes, print path, exit
fv --watch path/to/file --watch-timeout-secs 5
```

See [docs/CLAUDE_CODE.md](docs/CLAUDE_CODE.md) for the full list.

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
- [Performance comparison](docs/BENCHMARKS.md)


## License

MIT
