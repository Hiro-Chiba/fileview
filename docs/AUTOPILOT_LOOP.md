# Autopilot Loop Setup

This document sets up continuous local execution of the growth loop:

`market research -> implement -> compare score -> repeat`

## Philosophy Anchor

- Keep `philosophy.md` as the source of truth.
- Optimize for AI-driven development in the remaining ~20% terminal area.
- Escalate only major decisions.

## One-Time Setup (macOS)

From project root:

```bash
chmod +x scripts/ai_growth_loop.sh \
  scripts/install_growth_loop_launchd.sh \
  scripts/uninstall_growth_loop_launchd.sh \
  scripts/growth_loop_status.sh
```

Install and start launchd agent:

```bash
./scripts/install_growth_loop_launchd.sh 1800
```

- `1800` means one cycle every 30 minutes.
- Logs are written to `.automation/logs/`.

## Useful Commands

Run one cycle manually:

```bash
./scripts/ai_growth_loop.sh
```

Check status and recent logs:

```bash
./scripts/growth_loop_status.sh
```

Stop and uninstall:

```bash
./scripts/uninstall_growth_loop_launchd.sh
```

## Loop Outputs to Keep Updated

- `score.md`: latest score snapshot and delta
- `docs/COMPETITIVE_SCORECARD.md`: weekly market metrics
- `CHANGELOG.md` / release notes when shipped

## Notes

- `cargo audit` may fail in restricted environments due advisory DB lock permissions.
- CI may be unavailable due account billing limits; in that case rely on local checks and report explicitly.
