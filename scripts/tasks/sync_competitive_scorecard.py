#!/usr/bin/env python3
from __future__ import annotations

import datetime as dt
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SNAP = ROOT / "data" / "market_snapshot.json"
DOC = ROOT / "docs" / "COMPETITIVE_SCORECARD.md"

REPOS = [
    ("fileview", "Hiro-Chiba/fileview"),
    ("yazi", "sxyazi/yazi"),
    ("lf", "gokcehan/lf"),
    ("nnn", "jarun/nnn"),
    ("ranger", "ranger/ranger"),
]


def main() -> int:
    if not SNAP.exists():
        return 1
    snap = json.loads(SNAP.read_text())
    rows = []
    for key, name in REPOS:
        r = snap.get("repos", {}).get(key, {})
        rows.append(
            f"| {name} | {int(r.get('stargazers_count', 0)):,} | {int(r.get('forks_count', 0)):,} | {int(r.get('open_issues_count', 0)):,} |"
        )

    today = dt.date.today().isoformat()
    content = [
        "# Competitive Scorecard",
        "",
        f"Last updated: {today}",
        "",
        "## Market Snapshot",
        "",
        "| Repo | Stars | Forks | Open Issues |",
        "|---|---:|---:|---:|",
        *rows,
        "",
        "## Notes",
        "",
        "- Source of truth for loop score: `score.md`",
        "- Autopilot updates this file from `data/market_snapshot.json`",
    ]
    DOC.write_text("\n".join(content) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
