#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT_DIR/logs"
SENDER_NODE="${DEMO_SENDER_NODE:-sender}"
LSP_NODE="${DEMO_LSP_NODE:-lsp}"
RECIPIENT_NODE="${DEMO_RECIPIENT_NODE:-recipient}"

for name in lspd demo-ui "$SENDER_NODE" "$LSP_NODE" "$RECIPIENT_NODE"; do
  file="$LOG_DIR/$name.pid"
  if [ -f "$file" ]; then
    kill "$(cat "$file")" >/dev/null 2>&1 || true
    echo "[stop] $name"
  fi
done

for node in "$SENDER_NODE" "$LSP_NODE" "$RECIPIENT_NODE"; do
  pkill -f "fnn -c .*runtime/$node/config.yml" >/dev/null 2>&1 || true
  pkill -f "fnn .*runtime/$node/config.yml" >/dev/null 2>&1 || true
done
