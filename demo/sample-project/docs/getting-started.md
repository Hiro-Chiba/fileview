# Getting started

## Requirements

- Rust 1.90 or newer
- Node.js 22
- Docker (optional, for Postgres)

## Run everything locally

```sh
docker compose up -d db
make build
cargo run -p nebula-cli -- serve
npm --prefix web run dev
```

Then open <http://localhost:5173> and submit a job:

```sh
curl -X POST localhost:9090/jobs -d '{"name":"build","priority":9}'
```

## Next steps

- Read the [architecture guide](architecture.md)
- Tune the retry policy in `scripts/retry.lua`
