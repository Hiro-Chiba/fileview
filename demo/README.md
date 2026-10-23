# Demo recordings

The GIFs in `assets/` are recorded with [VHS](https://github.com/charmbracelet/vhs)
from the tapes in `tapes/`. Each tape runs `setup.sh`, which copies
`sample-project/` to `/tmp/nebula`, generates the binary assets (images, a
video, a PDF, and archives), and leaves a few Git changes behind.

You need VHS, ffmpeg, python3, and the FiraCode Nerd Font. Build the release
binary first, then run a tape from the repository root:

```sh
cargo build --release
vhs demo/tapes/demo.tape
```

| Tape | Output |
| --- | --- |
| `demo.tape` | `assets/demo.gif`, the overview at the top of the README |
| `preview.tape` | `assets/demo-preview.gif` |
| `markdown.tape` | `assets/demo-markdown.gif` |
| `search.tape` | `assets/demo-search.gif` |
| `git.tape` | `assets/demo-git.gif` |
| `files.tape` | `assets/demo-files.gif` |
| `navigation.tape` | `assets/demo-navigation.gif` |

The tapes share `/tmp/nebula`, so record them one at a time. Images use the
halfblocks protocol because it is the one that renders inside the VHS terminal.
