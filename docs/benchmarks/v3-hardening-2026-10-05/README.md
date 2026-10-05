# V3 follow-up hardening validation

This evidence covers contextual syntax refinement, bounded long-line search
result framing, and cross-platform checks. The source remains local and
unreleased. See [the design](../../WORKSPACE_ENGINE.md) for behavior and limits.

macOS passed 1181 default and 1127 minimal-feature tests. Linux aarch64 passed
1227 default and 1173 minimal all-target test executions. Each retains 16
existing ignored tests. Linux counts include example helper tests and a Linux
watcher regression test, so the counts are not directly comparable to macOS.
Formatting, both macOS all-target Clippy configurations, rustdoc with warnings
denied, and Rust 1.90 all-target checking also passed.

Both platforms ran actual terminal tests for workspace search, content result
navigation, forward/backward paging, and a 16MiB single line. Linux executed
both feature configurations with ripgrep 13; macOS used a release build and
ripgrep 15. A fully written 1GiB log was searched and previewed on macOS, with
sampled TUI RSS from 10.92 to 13.47MiB. Linux also passed the large-preview test
on a 10MiB actual file. These RSS samples exclude the child scanner and are not
continuous peak measurements or latency benchmarks.

The initial Linux terminal failure is retained. Reads produced access events,
which the previous mini debouncer converted into generic mutations. Preview
invalidation then reset the page. Filtering access events before bounded
mutation coalescing fixed the failure. Both final Linux PTY configurations and
a read-versus-write watcher regression test passed after the fix.

Windows GNU cross-checks passed for default and minimal features, including
all targets, on Rust 1.93.1 with MinGW GCC POSIX. These are compile/type checks,
not executable links or execution on Windows. Native runtime behavior is still
unverified. Local CI changes install ripgrep when absent and test both feature
configurations on Windows, Linux, and macOS. No workflow was dispatched or code
pushed to execute that CI.

Long-line smoke data records scanner-child RSS separately. Result output stays
roughly 100 to 130 bytes for the tested literals even on 128MiB lines. The child
still used roughly 143 to 271MiB. Forcing mmap did not reduce that cost, so it was
not adopted. Timing fields are uncontrolled one-shot smoke measurements, with
other validation active, and do not establish performance improvements.

The Docker source was mounted read-only then copied into an isolated non-root
workspace. Its 171-entry source manifest matches the final local sources.
The dedicated container, build artifacts, and temporary large files were removed.
Raw logs, the manifest, environment summary, and relevant hashes are included.

Reproduce macOS/Unix terminal checks from the repository root after a release build.
The large test needs more than 1.5GiB free disk space and cleans its temporary files.

```sh
cargo build --release --locked --bin fv
python3 scripts/workspace_pty_smoke.py --binary target/release/fv
python3 scripts/content_pty_smoke.py target/release/fv
python3 scripts/large_preview_pty.py target/release/fv --mib 1024
```
