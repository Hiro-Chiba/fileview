#!/usr/bin/env bash
set -euo pipefail

LABEL="com.fileview.growth-loop"
PLIST_PATH="$HOME/Library/LaunchAgents/${LABEL}.plist"

if [[ -f "$PLIST_PATH" ]]; then
  launchctl bootout "gui/$(id -u)" "$PLIST_PATH" >/dev/null 2>&1 || true
  launchctl unload "$PLIST_PATH" >/dev/null 2>&1 || true
  rm -f "$PLIST_PATH"
  echo "Removed: ${PLIST_PATH}"
else
  echo "Not installed: ${PLIST_PATH}"
fi

echo "Done."
