# Nebula

> A tiny distributed job scheduler with a live dashboard, written in Rust, Go, and TypeScript.

Nebula schedules **thousands of jobs per second** across a fleet of workers,
keeps a dependency graph of every task, and streams progress to a web UI.

## Features

- **Fast scheduler** built on a lock-free queue
- *Dependency graphs* with cycle detection
- Live dashboard with `WebSocket` updates
- Pluggable storage: SQLite, Postgres, or S3
- ~~XML config~~ TOML config only

## Quick start

```sh
cargo run -p nebula-cli -- serve --port 8080
open http://localhost:8080
```

## Components

| Component | Language | Path |
|-----------|----------|------|
| Core scheduler | Rust | `crates/core` |
| Gateway | Go | `services/gateway` |
| Dashboard | TypeScript | `web` |
| Analytics | Python | `ml` |

## Roadmap

- [x] Lock-free queue
- [x] Web dashboard
- [ ] Multi-region workers
- [ ] Plugin marketplace

---

See the [architecture guide](docs/architecture.md) for the details.
