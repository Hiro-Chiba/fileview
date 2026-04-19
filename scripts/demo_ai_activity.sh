#!/usr/bin/env bash
# Demo script for the AI activity reflection feature.
#
# Usage:
#   1) In terminal A, start the interactive fileview:
#      ./target/release/fv --follow-ai .
#   2) In terminal B, run this script:
#      ./scripts/demo_ai_activity.sh
#
# You should see the status bar in terminal A update to
#   [AI*] mcp: read_file <path>
# and the tree focus move to each file in turn.
#
# This script simulates a Claude Code session by piping MCP JSON-RPC
# requests into `fv --mcp-server` through stdin.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${FV_BIN:-$ROOT/target/release/fv}"

if [[ ! -x "$BIN" ]]; then
  echo "error: $BIN not found or not executable" >&2
  echo "hint: run 'cargo build --release' first" >&2
  exit 1
fi

pick() {
  local path="$1"
  cat <<JSON
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_file","arguments":{"path":"$path"}}}
JSON
}

# List a few real paths in the repo to drive the demo.
SAMPLES=(
  "Cargo.toml"
  "src/main.rs"
  "src/ai_activity/mod.rs"
  "src/app/event_loop.rs"
  "src/handler/key.rs"
  "src/render/ai_activity.rs"
)

echo "simulating an AI session against $BIN"
echo "(fileview root: $ROOT)"

# Build a combined stdin stream: one request per line.
{
  echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}'
  for rel in "${SAMPLES[@]}"; do
    pick "$ROOT/$rel"
    sleep 1
  done
} | "$BIN" --mcp-server "$ROOT" >/dev/null

echo "done — check terminal A"
