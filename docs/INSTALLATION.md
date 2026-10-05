# Installation

[Download a prebuilt binary](https://github.com/Hiro-Chiba/fileview/releases/latest)
for your OS. Rust is not required. Choose the archive with the matching filename
ending below, then extract it and run the included executable from a terminal.

| System | Archive filename ending |
| --- | --- |
| macOS, Apple Silicon | `aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `x86_64-apple-darwin.tar.gz` |
| Linux, x86-64 (GNU) | `x86_64-unknown-linux-gnu.tar.gz` |
| Windows, x86-64 | `x86_64-pc-windows-msvc.zip` |

The Linux binary requires glibc 2.39 or newer. For older glibc versions or
musl-based systems, use the [Cargo installation below](#install-with-cargo) to build for your environment.

### macOS and Linux

Save your downloaded archive as `fileview.tar.gz`. In the folder containing it, run:

```sh
tar -xzf fileview.tar.gz
./fv
```

To install for your user, run these commands from the same folder:

```sh
mkdir -p "$HOME/.local/bin"
install -m 755 ./fv "$HOME/.local/bin/fv"
export PATH="$HOME/.local/bin:$PATH"
fv
```

The `export` applies to the current terminal session. If `~/.local/bin` is not
already on your `PATH`, add that line to your shell's startup file to use `fv`
in new terminals. Repeat the download and `install` steps to update.

### Windows (PowerShell)

Save your downloaded archive as `fileview.zip`. In the folder containing it, run:

```powershell
Expand-Archive -Path .\fileview.zip -DestinationPath .\fileview
.\fileview\fv.exe
```

To use `fv` from any folder or with an MCP client, place the executable
in a directory on your `PATH`.

## Install with Cargo

If you already have Rust 1.90 or newer:

```sh
cargo install fileview --locked
fv
```

## Build options

The default build includes clipboard support, Lua plugins, archive previews, and
AI helpers. To build without those optional features:

```sh
cargo install fileview --locked --no-default-features
```

You can also choose individual features:

```sh
cargo install fileview --locked --no-default-features --features ai,clipboard,lua,archive
```

For a build that favors speed over binary size, use
`cargo install fileview --locked --profile release-fast`.

For Chafa image support, install libchafa 1.8 or newer, then run
`cargo install fileview --locked --features chafa`.

## Optional tools

File browsing, text previews, image previews, and filename search work without these tools.
Install only the tools for the features you want, then restart FileView.

| Feature | Required tools |
| --- | --- |
| Content search (`text:`) | ripgrep (`rg`) |
| Git status and diffs | `git` |
| Video thumbnails | FFmpeg's `ffmpeg` and `ffprobe` |
| Video duration, resolution, and codec information | FFmpeg's `ffprobe` |
| PDF pages and page counts | Poppler's `pdftoppm` and `pdfinfo` |

If a video or PDF preview is unavailable, check that the required tools are
installed and available on your `PATH`. Image previews adapt to your terminal
as described below.

## Image previews

Your terminal is auto-detected:

| Terminal | Protocol |
|----------|----------|
| Kitty / Ghostty / Konsole | Kitty Graphics |
| iTerm2 / WezTerm / Warp | iTerm2 Inline |
| Foot / Windows Terminal | Sixel |
| VS Code / Alacritty | Halfblocks |

See the [configuration guide](CONFIGURATION.md) to choose a preview protocol manually.
