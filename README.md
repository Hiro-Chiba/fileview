# FileView (fv)

[![Crates.io](https://img.shields.io/crates/v/fileview.svg)](https://crates.io/crates/fileview)
[![Downloads](https://img.shields.io/crates/d/fileview.svg)](https://crates.io/crates/fileview)
[![CI](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml/badge.svg)](https://github.com/Hiro-Chiba/fileview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.90-blue.svg)](https://www.rust-lang.org)

> Browse files, preview their contents, and search your project from the terminal.

<p align="center">
  <img src="assets/demo.gif" alt="FileView terminal file browser demo" width="80%">
</p>

FileView shows text and image previews alongside your files, with Git status and
keyboard or mouse navigation. No configuration is needed to get started.

## Install

[Download a binary for macOS, Linux, or Windows](https://github.com/Hiro-Chiba/fileview/releases/latest).
You don't need Rust. The [installation guide](docs/INSTALLATION.md) covers which
archive to choose, how to run it, and how to add `fv` to your `PATH`.

If you have Rust 1.90 or newer:

```sh
cargo install fileview --locked
```

## Get started

Run `fv` in a project folder, or pass a path:

```sh
fv ./my-project
```

| Key | Action |
| --- | --- |
| `j` / `k` or up/down arrows | Move through files |
| `h` / `l` | Collapse or expand a directory |
| `P` | Toggle the preview panel |
| `Ctrl+P` | Find a file or search its contents |
| `?` | Show help |
| `q` | Quit |

## Search and preview

Press `Ctrl+P` and type a filename. Use `ext:rs git:changed` to find Rust files
with Git changes, or `text:TODO` to search inside files. Select a content result
and press Enter to open the matching line and column.

Content search needs [ripgrep](https://github.com/BurntSushi/ripgrep) (`rg`) on your
`PATH`. It matches literal text, is case-sensitive, and cannot be combined with
filename filters. Filename search works without `rg`. Search respects project
ignore rules, including `.gitignore` and `.ignore`.

The same queries work from the terminal, with JSON output for scripts:

```sh
fv search main
fv search 'ext:rs git:changed' . --json
fv search 'text:TODO' . --json
```

Text previews load pages as you scroll. With the preview focused, use `g` for
the beginning and `G` for the end. See the [3.0 release notes](CHANGELOG.md#300---2026-10-05)
for the changes and upgrade notes.

## More details

- [Installation, optional tools, and image support](docs/INSTALLATION.md)
- [All keybindings](docs/KEYBINDINGS.md) and [configuration](docs/CONFIGURATION.md)
- [Claude Code, MCP, and scripting](docs/CLAUDE_CODE.md)
- [Lua plugins](docs/PLUGINS.md)
- [Search and preview design](docs/WORKSPACE_ENGINE.md), including current limits
- [Performance measurements](docs/BENCHMARKS.md) and [changelog](CHANGELOG.md)

## License

[MIT](LICENSE)
