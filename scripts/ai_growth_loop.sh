#!/usr/bin/env bash
set -euo pipefail

# Backward-compatible entrypoint.
exec python3 "$(cd "$(dirname "$0")" && pwd)/autopilot_loop.py" "$@"
