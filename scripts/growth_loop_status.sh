#!/usr/bin/env bash
set -euo pipefail

LABEL="com.fileview.growth-loop"
PLIST_PATH="$HOME/Library/LaunchAgents/${LABEL}.plist"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_LOG="$ROOT_DIR/.automation/logs/growth-loop.out.log"
ERR_LOG="$ROOT_DIR/.automation/logs/growth-loop.err.log"

if [[ -f "$PLIST_PATH" ]]; then
  echo "plist: $PLIST_PATH"
else
  echo "plist not found: $PLIST_PATH"
fi

echo
echo "launchctl:"
if launchctl print "gui/$(id -u)/${LABEL}" >/dev/null 2>&1; then
  launchctl print "gui/$(id -u)/${LABEL}" | rg -n "state =|last exit code|pid =" || true
else
  launchctl list | rg "$LABEL" || echo "not loaded"
fi

echo
if [[ -f "$OUT_LOG" ]]; then
  echo "stdout tail:"
  tail -n 20 "$OUT_LOG"
else
  echo "stdout log not found: $OUT_LOG"
fi

echo
if [[ -f "$ERR_LOG" ]]; then
  echo "stderr tail:"
  tail -n 20 "$ERR_LOG"
else
  echo "stderr log not found: $ERR_LOG"
fi
