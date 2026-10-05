# Content search and bounded preview implementation validation

This folder records the local implementation following the separate feasibility
experiments. See [WORKSPACE_ENGINE.md](../../WORKSPACE_ENGINE.md) for behavior,
interfaces, and known limits. No PR, push, version bump, or release was performed.

The default suite passed 1169 tests, and the minimal-feature suite passed 1115.
Each configuration retains 16 existing ignored tests. Both all-target Clippy
runs, formatting, rustdoc with warnings denied, and Rust 1.90 all-target checks
passed. Raw logs and source hashes are preserved here.

The release TUI passed the existing workspace PTY suite, including true piped
stdin, and the new content PTY suite. The latter verifies a content result opens
the correct line despite an active tree filter, forward/backward page crossings,
first/last navigation, and replacement of search input.

A fully written, synced 1GiB log was searched through the actual release TUI.
Its final match opened on line 8,388,608, `g` returned to the first page, and a
subsequent `g` superseded an in-progress `G` scan. Sampled TUI RSS was 10.81MiB
before search, 11.56MiB afterward, 11.59MiB after opening the result, and 13.09MiB
after cancelling the end scan. These samples exclude the ripgrep child and are
not peak memory or latency benchmarks. The fixture was removed after the test.

Reproduce from the repository root on a Unix system with ripgrep installed.
All PTY scripts isolate user settings in a temporary HOME. The large-file script
creates actual bytes and cleans up its temporary directory; it needs more than
1.5GiB of free space for the 1GiB case.

```sh
cargo build --release --locked --bin fv
python3 scripts/workspace_pty_smoke.py --binary target/release/fv
python3 scripts/content_pty_smoke.py target/release/fv
python3 scripts/large_preview_pty.py target/release/fv --mib 1024
```
