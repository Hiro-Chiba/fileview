# Stability Policy

This project uses release channels:

- `*-alpha`: Fast iteration, features may evolve quickly.
- Stable (`x.y.z`): Recommended for general use.

## Alpha Exit Criteria

FileView can move from alpha to stable when all conditions are met:

1. CI remains green for at least 2 consecutive release cycles.
2. No critical incidents in those cycles:
   - crash loops
   - data-loss/corruption bugs
   - security fixes requiring emergency release
3. `cargo audit` is green under committed policy (`.cargo/audit.toml`), and any temporary ignores are documented.
4. CLI behavior is compatibility-focused (no unannounced breaking changes in existing flags/actions).

## Current Assessment (2026-02-04)

- CI: green
- Local quality gates: `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all pass
- Security: `cargo audit -q` passes with documented policy
- Critical bug scan: no active critical bug identified

Based on this, the project is eligible for stable release.
