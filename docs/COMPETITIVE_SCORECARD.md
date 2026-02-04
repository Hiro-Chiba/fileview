# Competitive Scorecard

Last updated: 2026-02-04

This file tracks market position and execution metrics for `fileview` against core terminal file manager competitors.

## Scope

- Competitors: `yazi`, `lf`, `nnn`, `ranger`
- Update cadence: weekly
- Goal: move from niche AI-oriented utility to broadly trusted daily driver

## Current Snapshot (2026-02-04)

| Metric | fileview | yazi | lf | nnn | ranger |
|---|---:|---:|---:|---:|---:|
| GitHub stars | 0 | 32,179 | 9,026 | 21,191 | 16,834 |
| GitHub forks | 0 | 701 | 359 | 798 | 923 |
| Open issues | 0 | 73 | 66 | 2 | 921 |
| Latest release tag | v2.2.2 | v26.1.22 | r41 | v5.1 | (no latest release API) |
| crates.io downloads | 596 | - | - | - | - |

Notes:
- `fileview` now has release records for `v2.0.2-alpha` through `v2.2.2` to align tags and Releases.
- `ranger` currently returns 404 for GitHub "latest release" API endpoint.

## Product Position (Honest)

| Axis | fileview today | Gap to close |
|---|---|---|
| AI workflow integration | Strong differentiator (MCP/context-pack/select flows) | Need broader awareness and case studies |
| Zero-config onboarding | Strong | Keep startup and setup friction low |
| Ecosystem/network effects | Weak | Grow contributors, examples, third-party coverage |
| Community trust signals | Weak | Improve release notes, reliability signals, docs depth |
| Power-user extensibility | Medium | Expand plugin references and recipes |

## 90-Day Targets

| KPI | Baseline | Target |
|---|---:|---:|
| GitHub stars | 0 | 150 |
| External contributors (rolling 90d) | 0 | 10 |
| crates.io weekly downloads | 596 (recent total snapshot) | 300+/week |
| AI workflow case studies | 0 curated | 10 |
| Release note coverage for new tags | partial before sync | 100% |

## Weekly Review Template

Copy this block every week:

```md
## Week YYYY-MM-DD
- Product:
  - shipped:
  - regressions:
- Growth:
  - stars:
  - downloads:
  - contributors:
- Competitive:
  - yazi:
  - lf:
  - nnn:
  - ranger:
- Decision:
  - double-down:
  - de-prioritize:
```

## Source Links

- fileview: <https://github.com/Hiro-Chiba/fileview>
- yazi: <https://github.com/sxyazi/yazi>
- lf: <https://github.com/gokcehan/lf>
- nnn: <https://github.com/jarun/nnn>
- ranger: <https://github.com/ranger/ranger>
- crates.io fileview: <https://crates.io/crates/fileview>
