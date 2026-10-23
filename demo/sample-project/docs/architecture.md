# Architecture

Nebula has three moving parts. The **gateway** accepts jobs, the **scheduler**
decides where they run, and the **dashboard** shows what is happening.

```
 client ──▶ gateway (Go) ──▶ scheduler (Rust) ──▶ workers
                                  │
                                  ▼
                           dashboard (TS)
```

## Scheduler

The scheduler keeps a `TaskGraph` of every job and only queues a job once all
of its dependencies are done.

1. A job arrives with a priority and a list of dependencies
2. The graph rejects cycles before anything is queued
3. Ready jobs go into a priority queue that keeps *FIFO order* within a priority
4. Each tick hands out as many jobs as there are free slots

> Jobs are never dropped silently. A timed out job is reported back to the
> gateway, which decides whether to retry it.

## Storage

| Backend | Use case | Durable |
|---------|----------|---------|
| SQLite | Local development | Yes |
| Postgres | Production | Yes |
| Memory | Tests | No |

## Design decisions

- **Rust for the hot path**: no GC pauses while dispatching
- **Go for the gateway**: simple HTTP and gRPC tooling
- **Lua for retry policies**: operators can change them without a rebuild
