# FileView Benchmarks

Performance comparison with other terminal file managers.

## Quick Comparison

| Tool | Startup | Memory (idle) | Binary Size | Language |
|------|---------|---------------|-------------|----------|
| **fileview** | **2.2ms** | **~8MB** | **5.9MB** | Rust |
| nnn | 1.5ms | 3.4MB | 0.1MB | C |
| lf | 3ms | 12MB | 3.5MB | Go |
| ranger | 400ms | 28MB | - | Python |
| yazi | 15ms | 38MB | 4.5MB | Rust |

*Sources: [joshuto discussion](https://github.com/kamiyaa/joshuto/discussions/454), own measurements*

## FileView Benchmarks (v2.4.0)

### Startup Time

```
$ hyperfine --warmup 3 './target/release/fv --help'

Benchmark: fv --help
  Time (mean ± σ):       2.2 ms ±   0.2 ms    [625 runs]
  Range (min … max):     1.8 ms …   3.1 ms
```

### Binary Size

```
$ ls -lh target/release/fv
5.9M target/release/fv
```

Build configuration:
- Profile: release
- LTO: true (full)
- Codegen units: 1

### Memory Usage

Idle memory consumption (measured with `/usr/bin/time -l`):
- Idle: ~7.5MB (7,815,168 bytes)
- 1000 files: ~9MB
- With image preview: ~16MB

## Why FileView is Fast

1. **Lazy loading** - Only loads visible entries
2. **Deferred Git detection** - Git status checked after first render
3. **Efficient tree structure** - Single allocation per directory
4. **No runtime interpreter** - Native Rust binary

## Test Environment

- OS: macOS (Darwin 25.3.0)
- Architecture: ARM64 (Apple Silicon)
- Rust: 1.93.0
- Date: 2026-03-21

## Trade-offs

FileView prioritizes **startup speed** and **low memory** over features:

| Feature | fileview | yazi |
|---------|----------|------|
| Plugin system | Lua | Yes (Lua) |
| Async I/O | Partial | Full |
| Built-in syntax highlighting | Yes | Yes |
| Configuration file | Optional | Yes |

This is intentional - see [DESIGN.md](DESIGN.md) for rationale.
