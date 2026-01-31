# FileView Benchmarks

Performance comparison with other terminal file managers.

## Quick Comparison

| Tool | Startup | Memory (idle) | Binary Size | Language |
|------|---------|---------------|-------------|----------|
| **fileview** | **2.3ms** | **~8MB** | **2.9MB** | Rust |
| nnn | 1.5ms | 3.4MB | 0.1MB | C |
| lf | 3ms | 12MB | 3.5MB | Go |
| ranger | 400ms | 28MB | - | Python |
| yazi | 15ms | 38MB | 4.5MB | Rust |

*Sources: [joshuto discussion](https://github.com/kamiyaa/joshuto/discussions/454), own measurements*

## FileView Benchmarks (v1.14.3)

### Startup Time

```
$ hyperfine --warmup 3 './target/release/fv --help'

Benchmark: fv --help
  Time (mean ± σ):       2.3 ms ±   0.5 ms
  Range (min … max):     1.6 ms …   8.9 ms
```

### Binary Size

```
$ ls -lh target/release/fv
2.9M target/release/fv
```

Build configuration:
- Profile: release
- LTO: thin
- Codegen units: 1

### Memory Usage

Idle memory consumption in typical usage:
- Empty directory: ~6MB
- 1000 files: ~8MB
- With image preview: ~15MB

## Why FileView is Fast

1. **Lazy loading** - Only loads visible entries
2. **Deferred Git detection** - Git status checked after first render
3. **Efficient tree structure** - Single allocation per directory
4. **No runtime interpreter** - Native Rust binary

## Test Environment

- OS: macOS (Darwin 24.6.0)
- Architecture: ARM64 (Apple Silicon)
- Rust: 1.75+
- Date: 2026-01-31

## Trade-offs

FileView prioritizes **startup speed** and **low memory** over features:

| Feature | fileview | yazi |
|---------|----------|------|
| Plugin system | No | Yes (Lua) |
| Async I/O | Partial | Full |
| Built-in syntax highlighting | Yes | Yes |
| Configuration file | No | Yes |

This is intentional - see [DESIGN.md](DESIGN.md) for rationale.
