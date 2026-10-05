# Workspace engine experiment

This work evaluates a shared metadata index for interactive, CLI, and MCP searches.
It is an experimental local branch, not a released FileView 3.0 feature.

## Validation plan

Compare the existing filesystem walk and fuzzy matcher with a reusable in-memory
index and a persisted metadata cache. Measure scan time, search p50 and p95,
cache loading, incremental updates, and process memory separately. Run the same
queries against 100,000 files and approximately one million files. The 50 ms p95
search goal applies to a completed index, and must not be presented as cold-start
or complete TUI input-to-display latency.

Check ranked results against the existing matcher. Check incremental updates
against an independent full rebuild after file changes, directory moves, and
bursts. Exercise ignored and hidden files, Unicode paths, symlinks, invalid
queries, corrupt caches, cancellation, notification overflow recovery, and
reopening after changes made while FileView was stopped.

## Candidate design

A lazy workspace service owns a path/type index and bounded search mailboxes.
The UI never walks a workspace or performs a search on its event thread. A newer
query cancels the older query, and a new index revision refreshes the current
query. CLI and MCP searches use the same index and filter parser. Equal scores
are ordered by relative path, which can differ from the old traversal order.

The selected candidate stores paths and types in a BTreeMap. It reads size and
modification time only for the highest-ranked matches. The initial eager-metadata
candidate required a filesystem metadata call for every indexed path. Neither
candidate stores file contents. Local `.gitignore` and `.ignore` rules apply, `.git` is excluded, and
directory symlinks are not traversed. Hidden metadata is indexed but only shown
when requested. Git predicates use current Git status independently of file
metadata.

Recursive filesystem notifications supply changed paths. An overflowing event
queue or explicit refresh requests a full reconciliation. A five-minute scan
provides recovery from missed native notifications. When native watching is not
available, the fallback interval is five seconds. Results are eventually
consistent, not a filesystem transaction or an immediate read-after-write guarantee.
A winner that disappears or no longer satisfies its type filter is omitted.
Concurrent changes can temporarily return fewer than the requested number of
results until reconciliation. Metadata errors are reported rather than hidden.
After an indexing error, reconciliation retries at the five-second interval.

The default service uses memory only. The optional library cache remains an
experimental comparison, and its results are provisional until startup
reconciliation succeeds. It is not enabled by the CLI or TUI. Cache files within
the workspace are rejected to prevent cache writes from triggering more updates.

## Search commands

```sh
fv search main
fv search 'ext:rs git:changed' ./src --json
```

`QUERY` is required and the root defaults to the current directory. `--json`
returns structured results, `--hidden` includes hidden paths, and `--limit`
accepts 1 to 1000. Queries are limited to 4096 bytes. The index refuses more than
one million entries explicitly rather than dropping results silently.

The MCP tool is `search_workspace`. It requires `query` and optionally accepts
`show_hidden` and `limit`. It searches only the server's workspace root. CLI,
MCP, and Ctrl+P share the same filename filter syntax and index implementation.
Queries beginning with `text:` use the separate streaming content scanner below.

Ctrl+P uses this service while stdin-based selection retains its original matcher.
The workspace root is isolated when switching tabs or directories. Matching
respects project-local ignore rules, unlike the previous Ctrl+P recursive walk.
The `git:changed`, `git:modified`, and `git:untracked` filters select existing
indexed paths. Deleted Git paths are not synthesized into the filesystem listing.
`type:file` includes non-directory entries such as symlinks. JSON output reports
an error for paths that cannot be represented as UTF-8, while interactive search
retains the original filesystem path.

## Public interface simplified on 2026-10-05 JST

The public entry points are Ctrl+P, `fv search QUERY [PATH]`, and the
`search_workspace` MCP tool with a required query. Saved-query management was
removed to keep the interface focused on search. The measurements and validation
reports from 2026-10-04 remain unchanged as historical evidence for the earlier
prototype. Their test counts include saved-query scenarios that are no longer
part of the current interface.

The simplified interface passed macOS default and minimal test suites, Clippy
for both configurations, rustdoc, Rust 1.90 compilation, and the terminal smoke
test. An actual CLI/MCP comparison confirmed matching results and the query-only
MCP schema. The updated CLI checks cover plain queries, optional filters, JSON,
visibility, limits, missing arguments, and dash-prefixed queries using `--`.
[Interface validation](benchmarks/workspace-2026-10-05/interface-validation.json)
records this pass. The search algorithm was unchanged, so no performance
benchmark was repeated for this interface change.

## References

[Yazi's asynchronous previews](https://yazi-rs.github.io/docs/plugins/overview/)
and [Watchman's change tracking](https://facebook.github.io/watchman/) inform the
separation between interactive work and background filesystem work. File
selection follows the [ignore crate](https://docs.rs/ignore/latest/ignore/) used
by established Rust search tools. New performance claims require the measurements
from this experiment, rather than comparisons between unrelated benchmarks.

## Historical verification recorded on 2026-10-04 JST

The macOS validation used Apple M4, 16 GiB RAM, macOS 27.0.1, Rust 1.98.1,
and the shipping `release` profile (`opt-level = "z"`). The filesystem cache was
warm. The independent-query comparison resets the old matcher's incremental
state for each query. It does not claim a speedup for every sequence of typed
characters. These measurements cover engine operations, not complete UI latency
or cold storage. The large fixtures have no Git repository. Git predicates have
functional coverage, but million-file Git status, network filesystems, and cold
disk behavior have not been benchmarked. Raw measurements and machine details are in
[benchmarks/workspace-2026-10-04](benchmarks/workspace-2026-10-04/).

At 100,000 files plus 100 directories, 40 samples for each of eight queries gave
11.584 ms pooled search p95 for the final path/type engine, compared with
32.345 ms for the original matcher. Index initialization p95 was 149.035 ms,
compared with 269.515 ms for the first eager-metadata prototype. The original
plain walk took 52.674 ms, so the new engine does not improve first-open readiness.
Its benefits are background work, reuse, bounded results, and incremental updates.
A single-file reconciliation took 1.059 ms p95, and a 1,000-file directory
creation/deletion took 3.333 ms p95. Filesystem mutations are excluded from those
update timings. Current process RSS rose from approximately 7.4 MiB to 27.4 MiB
on the first build. This is not a general memory guarantee.

At 998,000 files plus 100 directories, the final engine completed ten samples
per query. Search p95 was 138.32 ms across 80 samples, so this workload does not
meet a 50 ms search target. Initialization p50 was 1,641.24 ms and p95 was
1,647.31 ms. With only ten initialization samples, that p95 is the maximum sample.
Single-file updates took 1.393 ms p95 and 1,000-file directory updates took
3.711 ms p95. First-build current process RSS was approximately 203 MiB,
including approximately 7.4 MiB before construction. Later benchmark memory
also includes repeated allocations and full correctness references.

An earlier combined million-file benchmark hit its 2 GiB process guard and
stopped safely. Because that run mixed baseline and index phases and emitted no
phase results, it cannot establish an engine-specific memory failure. The
subsequent isolated path candidate and final engine completed successfully.

A sorted Vec candidate reduced its query p95 to 10.876 ms and construction to
126.754 ms in a separate run. That small gain did not justify replacing the
BTreeMap's existing incremental update strategy. The Vec experiment has no
incremental update implementation. JSON loading plus the required rescan was
slower than scanning from scratch in both measured optimization profiles, so
persistence remains disabled by default. Historical eager-metadata results
remain recorded as historical evidence rather than measurements of the final
implementation.

A separate typing experiment preserved the previous matcher's incremental state.
It ran 40 traces of 20 updates, including extensions, backspaces, clearing, and
switching queries. Across those 800 updates, the previous matcher had 51.551 ms
p95 and the new engine had 37.378 ms p95. The p95 sum of search computation per
trace fell from 398.506 ms to 243.549 ms. These are computation times, not human
typing or full UI latency. Some narrow extensions were slower in the new engine.
For example, `module_042/file_0000` took 0.0145 ms p95 with the old incremental
matcher and 7.111 ms with the new full-index scoring. The current choice therefore
improves the measured trace overall, but is not faster on every keystroke.

The final implementation stops early when a filter-only query has found enough
matches, because all accepted scores tie and the index is already path-sorted.
At 100,000 files, empty search, `ext:rs`, and `type:dir` took 0.0392 ms, 0.0412 ms,
and 0.1754 ms p95 respectively. The bounded results matched the prefix of a full
query. This optimization does not change the eight nonempty fuzzy query paths
used for the prior large-scale measurements.

Correctness checks compare scores with the previous matcher and complete
incremental results with independent rebuilds. Regression tests include 576
mixed filesystem operations, ignore-rule changes, descendant-only notifications,
directory replacement by symlinks, current winner metadata, vanished results,
1,000 ignored writes, concurrent search requests, corrupt caches, queue overflow,
and 12 concurrent saved-query writers in the earlier prototype. That saved-query
scenario is historical and was removed with the simplified interface on 2026-10-05.

The macOS default-feature suite passed 1,136 tests and the minimal suite passed
1,082 tests. Each configuration retained 16 existing ignored tests. Tests that
attempt invalid UTF-8 filenames cannot exercise that branch on this APFS setup.
Default/minimal Clippy with warnings denied, rustdoc with warnings denied, and
Rust 1.90 compilation passed.

Linux aarch64 validation used Rust 1.93.1, Git 2.39.5, Python 3.11.2, and a
non-root user in disposable containers. The default all-targets suite passed
1,136 tests and the minimal suite passed 1,082 tests, with 16 existing ignored
tests in each. Both configurations passed the terminal smoke test. Invalid
UTF-8 filenames were actually created and the relevant index, CLI, and MCP
checks passed. Container build artifacts and containers were removed.
[Linux validation](benchmarks/workspace-2026-10-04/linux-validation.json) and
[macOS validation](benchmarks/workspace-2026-10-04/macos-validation.json) record
the checked scope. No Windows execution has been performed. Linux MSRV and
doctests were not part of this container validation.

The terminal smoke test exercises structured search, cancellation/reopening,
Enter and preview, invalid queries, live file creation, hidden visibility, and
an actual stdin pipe. It reproduced a macOS input failure caused by kqueue
registration of `/dev/tty`. Enabling Crossterm's `use-dev-tty` poll backend fixed
that failure. Reproduce the checks in an isolated temporary HOME with:

```sh
cargo build --locked --bin fv
python3 scripts/workspace_pty_smoke.py
```

A real MCP server process also passed tool registration, repeated search after a
file was created, and invalid-limit rejection. This supplements handler-level
tests and confirms that the process retains and updates its workspace engine.


## Content navigation and bounded previews, 2026-10-05

The local implementation now accepts `text:LITERAL` in Ctrl+P, `fv search`, and
`search_workspace`. It is literal and case-sensitive; everything after the
prefix is the search text, including spaces and filter-looking words. Filename
queries and their filters remain unchanged. No additional shortcut or command
family is introduced. Stdin selection retains its filename-only behavior.

Content search requires ripgrep on PATH only when requested. It ignores ripgrep
configuration, respects normal ignore and hidden-file rules, does not follow
symlinks during directory traversal, and returns the first match on each matching
line. Results follow scanning order. Ctrl+P keeps 15 results, and CLI/MCP accept
up to 1000. The same `--hidden`, `--limit`, and `--json` options apply. A missing
ripgrep executable produces an error without disabling filename search.

CLI/MCP content entries contain `path`, one-based `line`, zero-based byte
`byte_offset`, and `text`, a bounded snippet starting at the match. MCP paths
are relative to its workspace. JSON rejects unrepresentable non-UTF8 paths;
the TUI retains raw Unix paths. MCP also reports `limit_reached` and
`changed_results`, the count of match records rejected after observable changes.
One request, one result snapshot, and one scanner process are active per TUI
content worker. A newer query cancels the previous process and discards its
results. Cancellation kills and reaps the managed process.

Selecting a result reveals its path, clears any tree filename filter, and opens
text at the matched column and line. This bypasses automatic Git diff and custom
preview selection. File identity/size/mtime and matching bytes are checked;
preview reading validates the stamp again. These checks are best effort, not
an atomic snapshot. Edits that restore metadata, or edits already buffered by
ripgrep before its first result is received, cannot provide strict snapshot
line-number guarantees. Search again after edits.

The scanner now sends NUL-terminated paths followed by decimal line, byte-column,
and byte-offset fields and the matched literal. It no longer serializes entire
matching lines as JSON. Escaping the literal, consuming the rest of its line,
and replacing the output with the captured literal yields one bounded result
per line, even for dense repetition. Paths are bounded to 64KiB, queries to
4096 bytes, and the pipe queue to eight records. Snippets are read separately
from the verified file and retain at most the larger of 512 bytes or the query.
Newlines and colons in Unix filenames do not interfere with the framing.

Ripgrep runs in text mode to avoid unframed binary warnings. Files containing
NUL in the first 64KiB are skipped by the parent. This is explicitly a bounded
binary heuristic; a later NUL may be included in search results. Navigation
supports UTF-8, and automatic transcoding is disabled to preserve byte offsets.
A matching line over 1MiB no longer fails because of the old JSON record limit.
This bounds the parent process, not ripgrep's own line buffers. A 128MiB line
used roughly 143 to 271MiB in the scanner child in local smoke measurements;
`--mmap` did not improve that and was not adopted. Extremely large single-line
files therefore still carry a child-memory cost.

Ordinary text previews now read at most 64KiB and 256 lines per window, with
8KiB cancellation checkpoints. Continuation offsets retain line numbers across
large single lines and UTF-8 boundaries. Existing scrolling and page keys load
previous/next windows without retaining an unbounded page history. `G` scans
with bounded memory to determine correct final line numbers; it is asynchronous
and `g` supersedes it. Ordinary first windows retain syntax highlighting.
Distant windows display readable text first, then receive background syntax
styles reconstructed from the same file version. Full source lines are parsed
from the beginning before styles are clipped to the displayed byte range, so
multiline comments, strings, and matches in the middle of a line remain correct.
The refinement preserves any scrolling performed while it was being prepared.

Syntax reconstruction retains bounded line buffers, reads at most 8MiB, and
stops on lines longer than 64KiB or more than 1024 active highlight scopes.
Its 200ms replay budget and cancellation checks are cooperative between work
units, not an interruption of an individual syntax-regex call. If a limit is
reached, the readable plain page remains and its title explains the fallback.
Only stamped first pages enter the existing 32-entry cache. New selections invalidate in-flight
text requests, including cache-hit and non-text selections.

This change does not make blocking OS calls interruptible or retrofit every
PDF, custom preview, archive, or Git operation with cancellation. Native copying
is unchanged. The version and release workflow remain unchanged. This is local,
unreleased implementation; no PR or push accompanies it.

Implementation validation is recorded in
[v3-integration-2026-10-05](benchmarks/v3-integration-2026-10-05/README.md).
Default/minimal suites passed 1169/1115 tests, each with 16 existing ignored
cases. A real release-TUI test searched and opened the last matching line of a
fully written 1GiB log, navigated both directions across pages, and superseded
an end scan. Sampled TUI RSS ranged from 10.81 to 13.09MiB; this excludes the
ripgrep child and is not a peak-memory guarantee.


## Follow-up hardening, 2026-10-05

Distant-page syntax reconstruction and bounded long-line result framing are now
implemented locally. Regression tests compare styles against whole-file
highlighting for multiline Rust comments, Python strings, Unicode matches,
CRLF, and partially visible final lines. Real TUI coverage includes a 16MiB
single-line search result opened at its matching column.

Linux runtime validation exposed a read-triggered refresh loop in the existing
file watcher. The mini debouncer erased event kinds, so inotify access events
became generic changes and reset preview navigation. The watcher now filters
access events before coalescing mutations in one atomic dirty flag. A fixed
500ms deadline retains live refresh during continuous writes. A Linux test
verifies that repeated reads do not refresh while a subsequent write does.

The CI test matrix now provisions ripgrep when missing and tests default and
minimal features on all three operating systems. Unix runners also execute
both terminal smoke tests for each feature configuration. These are local CI
configuration changes; no remote workflow was started and no source was pushed.
Windows native runtime validation still requires a Windows runner. Compile-only
checks, where available, are reported separately from actual execution.

Final evidence is in
[v3-hardening-2026-10-05](benchmarks/v3-hardening-2026-10-05/README.md).
macOS default/minimal tests passed 1181/1127 cases. Linux all-target runs passed
1227/1173 executions, with actual terminal tests for both feature configurations.
Windows GNU default/minimal all-target compile checks passed in an isolated
cross-toolchain environment; executable linking and native runtime were not
verified. The dedicated container and temporary large files were removed.
