# Performance Comparison

## Why FileView Exists

nnn deliberately optimizes for a very small core. FileView was created because I
wanted to keep much of that simplicity and speed without reducing the built-in
experience quite as far. A fresh installation should be useful immediately:
install it, run `fv`, and get navigation, search, Git status, and previews without
first assembling a configuration.

The goal is a practical balance, not the lowest benchmark number at any cost.

## Results

These results are a local snapshot, not a universal ranking. "Ready" is the time
from process start until a fixture filename was rendered in the terminal.

| Product | 1 file ready | RSS | 10,000 files ready | RSS | Executable |
| --- | ---: | ---: | ---: | ---: | ---: |
| [FileView 2.8.0](https://github.com/Hiro-Chiba/fileview/releases/tag/v2.8.0) | 33.40 ms | 11.96 MiB | 50.21 ms | 18.84 MiB | 5.83 MiB |
| [nnn 5.3](https://github.com/jarun/nnn/releases/tag/v5.3) | 27.60 ms | 2.42 MiB | 50.44 ms | 3.97 MiB | 0.17 MiB |
| [Yazi 26.8.15](https://github.com/sxyazi/yazi/releases/tag/v26.8.15) | 48.15 ms | 24.84 MiB | 78.30 ms | 43.89 MiB | 17.78 MiB |
| [Superfile 1.6.0](https://github.com/yorukot/superfile/releases/tag/v1.6.0) | 57.50 ms | 28.74 MiB | 93.81 ms | 34.58 MiB | 25.55 MiB |
| [ranger 1.9.4](https://github.com/ranger/ranger) | 99.45 ms | 32.05 MiB | 259.76 ms | 70.56 MiB | 1.84 MiB + Python |
| [broot 1.59.0](https://github.com/Canop/broot/releases/tag/v1.59.0) | 144.51 ms | 12.97 MiB | 256.42 ms | 24.34 MiB | 10.18 MiB |
| [lf r42](https://github.com/gokcehan/lf/releases/tag/r42) | 1,035.47 ms | 12.81 MiB | 1,124.41 ms | 23.48 MiB | 5.36 MiB |

In this run, FileView and nnn reached the 10,000-file listing at effectively the
same time. nnn remained substantially smaller, as expected from its intentionally
minimal core. Among the compared tools with a richer built-in, preview-oriented
experience, FileView had the lowest ready time and memory use in this test.

## Method

- Measured on 2026-08-31 with an Apple M4, 16 GiB RAM, macOS 26.6.2, and an
  ARM64 120 by 40 pseudo-terminal.
- Each tool used a fresh home and configuration directory. One warm-up run was
  followed by 10 one-file runs and 5 runs with 10,000 files. The table shows
  medians.
- RSS was sampled 1.5 seconds after startup for the small fixture and 2 seconds
  after startup for the 10,000-file fixture.
- Official ARM64 release executables were used where available. nnn was compiled
  from the official 5.3 source with its default build. ranger was installed from
  PyPI and therefore also requires the Python runtime.
- FileView used its default feature set. Its image protocol was fixed to
  halfblocks so the headless terminal did not need to answer a protocol query.
- nnn was measured with its default core interface and no live-preview plugin.
  Its much smaller memory and executable size should be read with that feature
  difference in mind.
- Yazi used `TERM=dumb` because the headless terminal could not answer its startup
  terminal-capability queries. This may favor Yazi compared with an interactive
  terminal run.
- broot presents a tree rather than the same interface model, so its result is not
  a strict like-for-like comparison.
- lf took about one second to display the listing in this particular pseudo-terminal
  setup. That result should not be generalized beyond this environment.

Startup time, filesystem state, terminal behavior, operating-system caches, and
enabled integrations can all change these numbers. The table is intended to show
the trade-off FileView targets, not to declare one file manager best for every
user.
