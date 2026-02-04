#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
QUEUE_PATH = ROOT / "tasks" / "queue.json"
STATE_PATH = ROOT / "data" / "autopilot_state.json"
SCORE_PATH = ROOT / "score.md"
SNAPSHOT_PATH = ROOT / "data" / "market_snapshot.json"
ESCALATION_PATH = ROOT / "tasks" / "escalations.md"

REPOS = {
    "fileview": "Hiro-Chiba/fileview",
    "yazi": "sxyazi/yazi",
    "lf": "gokcehan/lf",
    "nnn": "jarun/nnn",
    "ranger": "ranger/ranger",
}

FALLBACK_METRICS = {
    "fileview": {"stargazers_count": 0, "forks_count": 0, "open_issues_count": 0, "pushed_at": None},
    "yazi": {"stargazers_count": 32179, "forks_count": 701, "open_issues_count": 73, "pushed_at": None},
    "lf": {"stargazers_count": 9026, "forks_count": 359, "open_issues_count": 66, "pushed_at": None},
    "nnn": {"stargazers_count": 21191, "forks_count": 798, "open_issues_count": 2, "pushed_at": None},
    "ranger": {"stargazers_count": 16834, "forks_count": 923, "open_issues_count": 921, "pushed_at": None},
}

BASE_SCORES = {
    "fileview": {"product": 22, "ai_fit": 18, "reliability": 10},
    "yazi": {"product": 33, "ai_fit": 6, "reliability": 12},
    "ranger": {"product": 29, "ai_fit": 4, "reliability": 13},
    "nnn": {"product": 24, "ai_fit": 3, "reliability": 14},
    "lf": {"product": 24, "ai_fit": 3, "reliability": 13},
}


def run(cmd: list[str], check: bool = True, capture: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=ROOT,
        check=check,
        text=True,
        capture_output=capture,
    )


def branch_guard() -> None:
    branch = run(["git", "rev-parse", "--abbrev-ref", "HEAD"]).stdout.strip()
    if branch != "develop":
        print(f"[guard] branch must be develop, got: {branch}", file=sys.stderr)
        sys.exit(2)


def ensure_dirs() -> None:
    (ROOT / "tasks").mkdir(parents=True, exist_ok=True)
    (ROOT / "data").mkdir(parents=True, exist_ok=True)


def load_json(path: Path, default: dict) -> dict:
    if not path.exists():
        return default
    return json.loads(path.read_text())


def save_json(path: Path, obj: dict) -> None:
    path.write_text(json.dumps(obj, ensure_ascii=False, indent=2) + "\n")


def fetch_repo(repo: str) -> dict | None:
    for _ in range(3):
        cp = run(
            ["gh", "api", f"repos/{repo}", "--jq", "{stargazers_count,forks_count,open_issues_count,pushed_at}"],
            check=False,
        )
        if cp.returncode == 0:
            return json.loads(cp.stdout)
        time.sleep(1)
    return None


def fetch_market_snapshot() -> dict:
    previous = load_json(SNAPSHOT_PATH, {})
    snapshot = {"updated_at": dt.datetime.utcnow().replace(microsecond=0).isoformat() + "Z", "repos": {}}
    for key, repo in REPOS.items():
        data = fetch_repo(repo)
        if data is None:
            data = previous.get("repos", {}).get(key)
        if data is None:
            data = FALLBACK_METRICS[key]
        snapshot["repos"][key] = data
    save_json(SNAPSHOT_PATH, snapshot)
    return snapshot


def ecosystem_score(stars: int, max_stars: int) -> int:
    if max_stars <= 0:
        return 1
    ratio = math.log10(stars + 1) / math.log10(max_stars + 1)
    return max(1, min(15, round(ratio * 15)))


def momentum_score(pushed_at: str | None) -> int:
    if not pushed_at:
        return 1
    pushed = dt.datetime.fromisoformat(pushed_at.replace("Z", "+00:00"))
    now = dt.datetime.now(dt.timezone.utc)
    days = max(0, (now - pushed).days)
    if days <= 1:
        return 10
    if days <= 3:
        return 9
    if days <= 7:
        return 8
    if days <= 14:
        return 6
    if days <= 30:
        return 4
    return 2


def apply_boosts(scores: dict, state: dict) -> None:
    boost = state.get("fileview_boost", {})
    for key in ("product", "ai_fit", "reliability", "ecosystem", "momentum"):
        scores["fileview"][key] += int(boost.get(key, 0))


def build_scores(snapshot: dict, state: dict) -> dict:
    repos = snapshot.get("repos", {})
    stars = {k: int(v.get("stargazers_count", 0)) for k, v in repos.items()}
    max_stars = max(stars.values(), default=0)
    scores: dict[str, dict] = {}
    for key in BASE_SCORES:
        base = BASE_SCORES[key]
        repo_data = repos.get(key, {})
        eco = ecosystem_score(int(repo_data.get("stargazers_count", 0)), max_stars)
        mom = momentum_score(repo_data.get("pushed_at"))
        row = {
            "product": base["product"],
            "ai_fit": base["ai_fit"],
            "reliability": base["reliability"],
            "ecosystem": eco,
            "momentum": mom,
        }
        row["total"] = sum(row.values())
        scores[key] = row
    apply_boosts(scores, state)
    scores["fileview"]["total"] = sum(
        scores["fileview"][k] for k in ("product", "ai_fit", "reliability", "ecosystem", "momentum")
    )
    return scores


def render_score_md(snapshot: dict, scores: dict, delta: int) -> str:
    order = ["yazi", "ranger", "nnn", "lf", "fileview"]
    lines = [
        "# fileview Competitive Score (Snapshot)",
        "",
        f"Updated: {dt.date.today().isoformat()}",
        "",
        "## Score Model (100)",
        "",
        "- Product capability: 35",
        "- AI workflow fit (20% terminal constraint): 25",
        "- Reliability/release operation: 15",
        "- Ecosystem/community: 15",
        "- Growth momentum: 10",
        "",
        "## Current Scores (No Flattery)",
        "",
        "| Product | Total | Product | AI Fit | Reliability | Ecosystem | Momentum |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for key in order:
        s = scores[key]
        lines.append(
            f"| {key} | {s['total']} | {s['product']} | {s['ai_fit']} | {s['reliability']} | {s['ecosystem']} | {s['momentum']} |"
        )
    lines.extend(
        [
            "",
            "## Score Delta (this cycle)",
            "",
            f"- fileview total delta: {delta:+d}",
            "",
            "## Market Snapshot",
            "",
            "| Repo | Stars | Forks | Open Issues |",
            "|---|---:|---:|---:|",
        ]
    )
    for key, repo in REPOS.items():
        r = snapshot.get("repos", {}).get(key, {})
        lines.append(
            f"| {repo} | {int(r.get('stargazers_count', 0)):,} | {int(r.get('forks_count', 0)):,} | {int(r.get('open_issues_count', 0)):,} |"
        )
    lines.extend(
        [
            "",
            "## Loop Rule",
            "",
            "1. Market research update",
            "2. Implement one measurable improvement from tasks/queue.json",
            "3. Re-score in this file",
            "4. Record before/after delta",
            "5. Repeat",
            "",
            "If no score delta appears for 2 cycles, pivot theme immediately.",
        ]
    )
    return "\n".join(lines) + "\n"


def load_state() -> dict:
    return load_json(
        STATE_PATH,
        {
            "fileview_boost": {"product": 0, "ai_fit": 0, "reliability": 0, "ecosystem": 0, "momentum": 0},
            "last_total": None,
            "no_delta_cycles": 0,
            "completed_tasks": [],
        },
    )


def load_queue() -> dict:
    return load_json(QUEUE_PATH, {"tasks": []})


def write_escalation(task: dict) -> None:
    ESCALATION_PATH.parent.mkdir(parents=True, exist_ok=True)
    now = dt.datetime.utcnow().replace(microsecond=0).isoformat() + "Z"
    line = f"- {now} | {task['id']} | {task['title']} | kind=major | action required\n"
    if ESCALATION_PATH.exists():
        ESCALATION_PATH.write_text(ESCALATION_PATH.read_text() + line)
    else:
        ESCALATION_PATH.write_text("# Escalations\n\n" + line)


def execute_task(queue: dict, state: dict) -> str:
    tasks = queue.get("tasks", [])
    todo = next((t for t in tasks if t.get("status", "todo") == "todo"), None)
    if not todo:
        return "no_task"
    if todo.get("kind", "normal") == "major":
        write_escalation(todo)
        return "major"
    cmd = todo.get("command")
    if not cmd:
        todo["status"] = "blocked"
        todo["result"] = "missing command"
        save_json(QUEUE_PATH, queue)
        return "blocked"
    cp = run(["/bin/zsh", "-lc", cmd], check=False)
    if cp.returncode != 0:
        todo["status"] = "failed"
        todo["result"] = cp.stderr[-800:]
        save_json(QUEUE_PATH, queue)
        return "failed"
    todo["status"] = "done"
    todo["completed_at"] = dt.datetime.utcnow().replace(microsecond=0).isoformat() + "Z"
    task_boost = todo.get("boost", {})
    for k in ("product", "ai_fit", "reliability", "ecosystem", "momentum"):
        state["fileview_boost"][k] += int(task_boost.get(k, 0))
    state["completed_tasks"].append(todo["id"])
    save_json(QUEUE_PATH, queue)
    return "done"


def maybe_commit_push(no_commit: bool, no_push: bool) -> None:
    if no_commit:
        return
    run(["git", "add", "score.md", "tasks", "data", "docs/COMPETITIVE_SCORECARD.md"], check=False, capture=False)
    cp = run(["git", "diff", "--cached", "--name-only"], check=False)
    if not cp.stdout.strip():
        return
    stamp = dt.datetime.utcnow().strftime("%Y-%m-%d %H:%M:%SZ")
    run(["git", "commit", "-m", f"chore(autopilot): loop cycle {stamp}"], capture=False)
    if not no_push:
        run(["git", "push"], capture=False)


def run_cycle(no_commit: bool, no_push: bool) -> int:
    ensure_dirs()
    branch_guard()
    print("[autopilot] cycle start")
    state = load_state()
    queue = load_queue()
    before = int(state["last_total"] or 0)
    snapshot = fetch_market_snapshot()
    print("[autopilot] market research updated")
    task_result = execute_task(queue, state)
    print(f"[autopilot] task result: {task_result}")
    scores = build_scores(snapshot, state)
    after = int(scores["fileview"]["total"])
    delta = after - before if before else 0
    SCORE_PATH.write_text(render_score_md(snapshot, scores, delta))
    if task_result == "done":
        if delta == 0:
            state["no_delta_cycles"] += 1
        else:
            state["no_delta_cycles"] = 0
    state["last_total"] = after
    state["last_task_result"] = task_result
    state["last_cycle_at"] = dt.datetime.utcnow().replace(microsecond=0).isoformat() + "Z"
    save_json(STATE_PATH, state)
    maybe_commit_push(no_commit, no_push)
    print(f"[autopilot] fileview score: {after} (delta {delta:+d})")
    if task_result == "major":
        print("[autopilot] major decision required; escalation recorded.")
        return 10
    if state["no_delta_cycles"] >= 2:
        print("[autopilot] no score delta for 2 cycles. Pivot required.")
        return 11
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--loop", action="store_true")
    parser.add_argument("--sleep", type=int, default=1800)
    parser.add_argument("--no-commit", action="store_true")
    parser.add_argument("--no-push", action="store_true")
    args = parser.parse_args()

    if not args.loop:
        return run_cycle(args.no_commit, args.no_push)
    while True:
        code = run_cycle(args.no_commit, args.no_push)
        if code in (10, 11):
            return code
        time.sleep(args.sleep)


if __name__ == "__main__":
    raise SystemExit(main())
