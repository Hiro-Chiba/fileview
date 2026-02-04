#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK_PATH="$ROOT_DIR/.git/hooks/pre-push"

cat > "$HOOK_PATH" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

branch="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$branch" == "main" ]]; then
  echo "[pre-push guard] Push from main is blocked. Use develop/feature branches." >&2
  exit 1
fi
EOF

chmod +x "$HOOK_PATH"
echo "Installed pre-push guard at $HOOK_PATH"
