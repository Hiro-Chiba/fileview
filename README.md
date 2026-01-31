# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![Downloads](https://img.shields.io/crates/d/fileview.svg)](https://crates.io/crates/fileview)
[![CI](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml/badge.svg)](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.75-blue.svg)](https://www.rust-lang.org)

> Zero-config terminal file browser with automatic image preview

English | [日本語](README_ja.md)

## Why fv?

```
Lightweight ◄───────────────────────► Feature-rich

   nnn    lf    fv    ranger    yazi
  3.4MB  12MB  8MB    28MB     38MB
```

- **Zero config** - Install and run. No setup required.
- **Auto image preview** - Detects Kitty/iTerm2/Sixel/Halfblocks automatically
- **Fast** - 2.3ms startup, 8MB memory (vs ranger 400ms/28MB)
- **Batteries included** - Git status, syntax highlighting, PDF preview
- **Vim keybindings** - Navigate with j/k/h/l

## Quick Start

```bash
cargo install fileview
fv
```

## Features

| Feature | Description |
|---------|-------------|
| Tree navigation | Expand/collapse with vim keys |
| Multi-select | Batch operations on files |
| Preview panel | Text, images, archives, hex dump |
| File operations | Create, rename, delete, copy/paste |
| Fuzzy finder | `Ctrl+P` for quick search |
| Mouse support | Click, scroll, drag |
| Nerd Fonts | Icons enabled by default |

## Image Preview

FileView auto-detects your terminal:

| Terminal | Protocol |
|----------|----------|
| Kitty / Ghostty / Konsole | Kitty Graphics |
| iTerm2 / WezTerm / Warp | iTerm2 Inline |
| Foot / Windows Terminal | Sixel |
| VS Code / Alacritty | Halfblocks |

## Keybindings (Quick Reference)

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

**[Full keybinding list](docs/KEYBINDINGS.md)**

## CLI Options

```bash
fv [OPTIONS] [PATH]

Options:
  -p, --pick          Pick mode: output selected path(s)
  -f, --format FMT    Output format: lines, null, json
  --stdin             Read paths from stdin
  --on-select CMD     Run command on selection
  --choosedir         Output directory on exit
  --no-icons          Disable Nerd Fonts icons
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Cancelled (pick mode) |
| 2 | Runtime error |
| 3 | Invalid arguments |

## Shell Integration

```bash
# Add to .bashrc or .zshrc
fvcd() {
  local dir=$(fv --choosedir "$@")
  [ -n "$dir" ] && [ -d "$dir" ] && cd "$dir"
}
```

## Installation Options

```bash
# Standard install
cargo install fileview

# With Chafa support (better image quality on basic terminals)
brew install chafa  # or apt install libchafa-dev
cargo install fileview --features chafa
```

## Documentation

- [Keybindings](docs/KEYBINDINGS.md) - Complete keybinding reference
- [Comparison](docs/COMPARISON.md) - vs yazi, lf, ranger, nnn
- [Benchmarks](docs/BENCHMARKS.md) - Performance data
- [Security](docs/SECURITY.md) - Security model

## License

MIT
