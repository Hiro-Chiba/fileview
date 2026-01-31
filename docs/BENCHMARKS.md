# Fileview Benchmarks

Benchmark results for fileview v1.9.0 on macOS (Apple Silicon).

## Startup Time

Measured using [hyperfine](https://github.com/sharkdp/hyperfine) with 3 warmup runs.

### fv --help

```
Benchmark: ./target/release/fv --help
  Time (mean ± σ):       2.1 ms ±   0.1 ms
  Range (min … max):     1.9 ms …   2.9 ms
```

## Binary Size

Release build with LTO and strip enabled:

```
Optimization: size (opt-level = "z")
LTO: true
Strip: true
```

## Test Environment

- OS: macOS (Darwin 24.6.0)
- Architecture: ARM64 (Apple Silicon)
- Date: 2026-01-31

## Notes

- Startup time is consistently under 3ms
- Binary is optimized for size with full LTO
- No runtime dependencies required
