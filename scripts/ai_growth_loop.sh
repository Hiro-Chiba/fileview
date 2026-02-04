#!/usr/bin/env bash
set -euo pipefail

# Single-cycle (default) or repeat mode for the growth loop.
# Usage:
#   scripts/ai_growth_loop.sh
#   scripts/ai_growth_loop.sh --loop --sleep 1800

LOOP=0
SLEEP_SEC=1800

while [[ $# -gt 0 ]]; do
  case "$1" in
    --loop)
      LOOP=1
      shift
      ;;
    --sleep)
      SLEEP_SEC="${2:-1800}"
      shift 2
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

run_cycle() {
  local ts
  local branch
  ts="$(date '+%Y-%m-%d %H:%M:%S')"
  echo "[ai-growth-loop] cycle start: ${ts}"

  branch="$(git rev-parse --abbrev-ref HEAD)"
  if [[ "${branch}" == "main" ]]; then
    echo "[guard] Refusing to run on main branch. Switch to develop." >&2
    exit 2
  fi
  if [[ "${branch}" != "develop" ]]; then
    echo "[guard] Refusing to run on '${branch}'. Autopilot is develop-only." >&2
    exit 3
  fi

  echo "[1/4] Quality checks"
  cargo fmt --all -- --check
  cargo clippy --all-targets -- -D warnings
  cargo test --quiet
  cargo audit || true

  echo "[2/4] Competitive snapshot reminder"
  echo "Update score.md with latest market metrics and deltas."

  echo "[3/4] Implementation rule"
  echo "Pick one improvement tied to narrow AI workflow in 20% terminal space."

  echo "[4/4] Decision gate"
  echo "Escalate only for major decisions listed in philosophy.md."

  echo "[ai-growth-loop] cycle done"
}

if [[ "${LOOP}" -eq 1 ]]; then
  while true; do
    run_cycle
    echo "[ai-growth-loop] sleep ${SLEEP_SEC}s"
    sleep "${SLEEP_SEC}"
  done
else
  run_cycle
fi
