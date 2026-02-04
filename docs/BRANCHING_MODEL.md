# Branching Model

Updated: 2026-02-04

## Policy

- `main`: release-only
- `develop`: integration branch for day-to-day development
- `feature/*`: short-lived branches from `develop`
- Only the owner may approve `develop -> main` and release operations.

## Standard Flow

1. Create feature branch from `develop`
2. Implement and run checks
3. Open PR into `develop`
4. Merge into `develop` after review
5. Merge `develop` into `main` only with explicit owner approval
6. Tag + release from `main`

## Quality Gate (before merge)

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --quiet`
- `cargo audit` (or documented environment exception)

## Release Gate (develop -> main)

Escalate and require owner approval when:

- stable release cut
- breaking change in CLI/MCP
- security-sensitive change
- potential data-loss risk
