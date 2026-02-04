#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This installer currently supports macOS launchd only." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SLEEP_SEC="${1:-1800}"
LABEL="com.fileview.growth-loop"
PLIST_PATH="$HOME/Library/LaunchAgents/${LABEL}.plist"
LOG_DIR="$ROOT_DIR/.automation/logs"
OUT_LOG="$LOG_DIR/growth-loop.out.log"
ERR_LOG="$LOG_DIR/growth-loop.err.log"

mkdir -p "$LOG_DIR" "$HOME/Library/LaunchAgents"

cat > "$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/zsh</string>
    <string>-lc</string>
    <string>cd "$ROOT_DIR" &amp;&amp; ./scripts/ai_growth_loop.sh --loop --sleep ${SLEEP_SEC}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>${OUT_LOG}</string>
  <key>StandardErrorPath</key>
  <string>${ERR_LOG}</string>
</dict>
</plist>
EOF

if launchctl print "gui/$(id -u)/${LABEL}" >/dev/null 2>&1; then
  launchctl bootout "gui/$(id -u)" "$PLIST_PATH" >/dev/null 2>&1 || true
fi

if launchctl bootstrap "gui/$(id -u)" "$PLIST_PATH" >/dev/null 2>&1; then
  launchctl kickstart -k "gui/$(id -u)/${LABEL}" >/dev/null 2>&1 || true
else
  # Fallback for shells/environments where bootstrap is unavailable.
  launchctl unload "$PLIST_PATH" >/dev/null 2>&1 || true
  launchctl load "$PLIST_PATH"
fi

echo "Installed and started: ${LABEL}"
echo "plist: ${PLIST_PATH}"
echo "logs:"
echo "  $OUT_LOG"
echo "  $ERR_LOG"
echo
echo "Use scripts/growth_loop_status.sh to inspect status."
