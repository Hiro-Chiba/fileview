# V3 capability feasibility evidence

Recorded on 2026-10-05 JST. See [the decision report](../../V3_EXPERIMENTS.md).
These are isolated experiments, not released capabilities. Timings use warm
local storage and a size-optimized release build. No timed runs overlapped
builds or other benchmark jobs.

`viewport-*` contains 40-sample bounded read timings and offscreen rendered rows.
Only the 10 MiB case allocates a whole-file comparison, with 10 samples.
`content-*` contains 10-sample custom scanner timings and one-shot references.
`cancel-*` contains 40 cross-thread custom scanner cancellation trials.
`integrated-*` verifies custom search callbacks to offscreen matching-line frames.
`copy-*` contains five completed-copy trials, 30 cancellation trials, checksum
checks, and controller heartbeat measurements. Small-sample copy p95 is the
maximum, not a robust tail estimate.

`rg-repeated.json` compares ten full subprocess executions for each query.
`rg-streamed.json` measures ten line-buffered JSON subprocesses per query,
including first-result delivery and child RSS. `rg-handoff-cancel.json` verifies
stable-file offset handoff and 30 cancellation/reaping trials. It does not yet
provide production file-stamp or request-generation integration.

The `.time` files contain macOS `/usr/bin/time -l` resource measurements.
Whole-file baseline allocations affect process peak RSS, so content JSON also
records peak RSS before reference allocation. Viewport JSON samples live RSS
separately. These methods must not be treated as identical memory measurements.
Source hashes and machine details are in `environment.json`; logs preserve
successful tests, Clippy, formatting, release build, and MSRV checks.

## Reproduction

The scripts expect macOS, Python 3, Rust, and ripgrep. They write results into
this directory, replacing the recorded result files. Preserve this directory
elsewhere first if retaining the original evidence. They take an existing
fixture directory as their only argument and remove copy outputs automatically.
At least 3 GiB of free disk space is advisable. Do not run alongside other builds
or performance tests.

Build the four examples from the repository root.

```sh
cargo build --release --locked --example viewport_probe --example content_search_probe --example content_view_probe --example copy_probe
```

Create the real fixtures using this Python code, which prints their directory.
All files are fully written and synced. It does not create sparse files.

```python
import json, os, pathlib, tempfile
root = pathlib.Path(tempfile.mkdtemp(prefix="fv-v3-probes-"))
line = b"INFO 2026-10-05 ordinary log record ".ljust(127, b"x") + b"\n"
block = line * 8192
files = []
for size in (10, 100, 1024):
    length = size * 1024 * 1024
    path = root / f"log-{size}mib.log"
    with path.open("xb") as output:
        for _ in range(size):
            output.write(block)
        for offset, marker in ((2048, b"FV_NEAR_START"), (length - 2048, b"FV_NEAR_END")):
            output.seek(offset)
            output.write((marker + b" Japanese target").ljust(127, b" ") + b"\n")
        output.flush()
        os.fsync(output.fileno())
    files.append({"path": str(path), "bytes": length, "first_line": 17,
                  "last_hit_line": (length - 2048) // 128 + 1,
                  "last_hit_offset": length - 2048})
(root / "fixture.json").write_text(json.dumps({"root": str(root), "line_bytes": 128,
                                               "files": files}, indent=2))
print(root)
```

Run each script sequentially, replacing `FIXTURE_DIRECTORY` with that path.
The final script tests process cancellation and matching-line handoff.

```sh
python3 docs/benchmarks/v3-capabilities-2026-10-05/run.py FIXTURE_DIRECTORY
python3 docs/benchmarks/v3-capabilities-2026-10-05/rg_compare.py FIXTURE_DIRECTORY
python3 docs/benchmarks/v3-capabilities-2026-10-05/rg_stream.py FIXTURE_DIRECTORY
python3 docs/benchmarks/v3-capabilities-2026-10-05/rg_handoff.py FIXTURE_DIRECTORY
```

Afterward remove only the three fixture logs and their `fixture.json`, then
remove the empty fixture directory. The original validation fixtures were
removed after all measurements, leaving only these results and experiment code.
