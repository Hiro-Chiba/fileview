# FileView 3.0 capability experiments

This report records the isolated feasibility experiments. A subsequent local
implementation is described in [WORKSPACE_ENGINE.md](WORKSPACE_ENGINE.md#content-navigation-and-bounded-previews-2026-10-05).
The measurements below remain historical prototype results, not production
performance claims.

Status as of 2026-10-05 JST: feasibility validation completed. These are isolated examples,
not additional product commands or shipped capabilities. The public interface
remains Ctrl+P, `fv search`, and the existing MCP tools. No PR, push, or release
is part of this experiment.

## Questions and acceptance criteria

1. Can streaming content search return a bounded result before scanning finishes,
   and open the correct matching line without reading the entire file again?
   Check literal matches, UTF-8, CRLF, chunk boundaries, long lines, ignore rules,
   binary policy, stale files, and cancellation. Compare equivalent searches with
   installed ripgrep. Prefer reusing a proven scanner if a new one is slower or
   requires excessive complexity.
2. Can the first text page and a page at a known matching offset be displayed with
   memory bounded independently of file size? The warm-filesystem target is
   p95 below 50 ms for page preparation, not complete terminal latency. Test
   missing final newlines, giant single lines, UTF-8 boundaries, truncation,
   replacement, and obsolete work. Existing whole-file preview remains a
   comparison only on a safe small fixture.
3. Can a background copy be cancelled without modifying its source or publishing
   a partial destination? Check collision, injected write failure, cancellation,
   source changes, and progress reporting. A file is committed individually;
   batch transactions, general undo, restart recovery, and cross-device moves
   are outside this prototype.

A cancellation p95 below 50 ms on ordinary warm local files is the initial target.
It does not promise that a blocked filesystem or network request is interruptible.
The memory target is bounded growth, with prototype RSS below 128 MiB for a 1 GiB
input. It is not a limit on the complete FileView application.

## Method

Use real 10 MiB, 100 MiB, and 1 GiB files written in chunks, not sparse files.
Each ordinary log line contains 128 bytes. Known markers occur near the beginning
and end. Filesystem caches are warm after fixture creation. Time first result,
complete scans, page preparation, and cancellation separately. Capture process
RSS separately from fixture generation and whole-file comparison allocations.
Run timed experiments sequentially, without concurrent builds or tests.

Compare logical `std::fs::copy` timing with streaming copy carefully. APFS may use
copy-on-write, so it is not an equal physical-byte throughput comparison. Inject
write failures in tests instead of filling the user's disk. Temporary input and
output data must be removed after measurement.

## Current implementation gaps

The existing preview worker already runs several formats in the background and
discards queued obsolete requests. It still reads and highlights entire text
files, and cannot cancel a currently running request. PDF/custom preview and
some directory expansion work remain synchronous.

MCP already has `search_code`, while TUI search does not navigate from a content
match to its matching line. The proposed change is a shared, bounded, streaming
flow rather than a second unrelated search interface.

File paste currently executes copy/move operations synchronously. Copies write
directly to their final destination, and moves use `rename`. Trash support exists;
general undo and crash recovery do not form part of this validation.

## Results and decision

Measurements use Apple M4, 16 GiB RAM, macOS 27.0.1, APFS, Rust 1.98.1,
and the shipping size-optimized release profile. The filesystem cache was warm.
Raw measurements, source hashes, commands, and test logs are in
[v3-capabilities-2026-10-05](benchmarks/v3-capabilities-2026-10-05/README.md).

### Prioritize bounded preview and search-to-line navigation

For a real 1 GiB file, preparing the first 40 lines took p95 0.0418 ms and
opening the known near-end matching offset took p95 0.0323 ms, each over 40
samples. Observed process RSS after window reads was 7.17 MiB. The first page
read 8 KiB, while the near-end page read only the remaining 2 KiB. These are
plain UTF-8 data preparation times, not terminal frame latency or syntax
highlighting times.

The 10 MiB whole-read plus existing preview constructor baseline took p95
13.56 ms over 10 samples, with maximum sampled live RSS 32.39 MiB. The `.log`
fixture does not exercise complex syntax. This is evidence for eliminating
file-sized allocations, not an equivalent syntax-highlighting speed comparison.

The integrated prototype rendered line 17 before its 1 GiB scan finished,
and correctly rendered the near-end match at line 8,388,593. UTF-8, CRLF,
missing final newline, large single lines, horizontal alignment, wide queries,
stale offsets, replacement, and cancellation have regression coverage. A
200-character prefix initially hid the match despite successful reading;
horizontal alignment fixes that failure in the experiment.

### Reuse a mature content scanner

Reject the custom scanner as the default engine candidate. Its 1 GiB complete
scan p95 was 1,000 to 1,476 ms across three queries, versus 261 to 307 ms for
the external ripgrep streaming experiment, each with 10 samples. With 10
samples, p95 is simply the maximum and should not be treated as a stable tail
estimate. The custom scanner remains a useful bounded-memory reference and
returns a near-start match sooner than a new subprocess can start.

For `rg --line-buffered --json`, near-start first-result p95 was 12.06 ms,
with measured child peak RSS below 4.55 MiB on this fixture. Default pipe
buffering delayed delivery until near completion, so enabling line buffering
is a material requirement. External-process cancellation and reaping took
p95 0.752 ms over 30 trials. Its first and near-end offsets were successfully
handed to the bounded offscreen viewport. The custom scanner's cooperative
cancellation p95 was 0.064 ms over 40 trials, but that does not cancel a blocked
kernel call.

The direction is streaming search results plus bounded matching-line preview,
with a proven scanner behind the existing interaction. This does not yet
choose between a managed `rg` process and its reusable Rust libraries. The
former was measured; embedded-library integration was not. A mandatory new
external dependency, fallback behavior, and supported encodings require a
product decision before implementation.

### Keep background copy separate from the initial adoption

The single-file prototype passed partial-write failure, accepted cancellation,
destination collision, source replacement, and cancellation-versus-publication
race tests. All completed copies matched the source checksum, the source was
unchanged, and ordinary cancellation left neither a final nor a temporary copy.
For 1 GiB, cancellation p95 was 2.261 ms over 30 trials. A standalone controller
continued its heartbeat with p95 1.270 ms during background copies; this is not
proof of actual TUI responsiveness.

Do not replace native copying with this streaming loop. On APFS, median logical
`std::fs::copy` time was 0.297 ms versus 886 ms for the background streamed copy,
over five trials. The native call can clone data, while the prototype physically
writes and syncs it. Background scheduling is still useful, but preserving
native fast paths and metadata needs a separate design. This is deferred from
the first two capabilities.

## Verification and remaining scope

All four example test targets passed with default and minimal features, 45
executions per configuration. Shared helper tests run in multiple targets, so
this is not 45 distinct cases. The default library suite passed 689 tests with
three pre-existing ignored cases. Formatting, all-target Clippy with warnings
denied in both configurations, and Rust 1.90 example checks passed.

The simplest adoption is to improve the existing picker and preview rather
than introduce another command family. No product command, TUI binding, PR,
push, or release was added for these experiments.

Before production adoption, integrate request generations and worker lifecycle,
bounded result queues, stale-file revalidation, syntax context, and a coherent
encoding/binary policy. Prototype metadata stamps are not atomic snapshots and
cannot detect edits that restore size and timestamp. Files replaced by special
files during opening can block; regular-file validation and platform-specific
opening need hardening. The prototype does not support arbitrary distant line
numbers without an offset, hard cancellation of OS calls, cold/network storage
latency guarantees, permissions/ACL preservation, directory transactions,
restart recovery, or general undo. New capabilities were validated on macOS;
Linux and Windows production integration remain unverified.
