#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT_DIR/logs"

for name in lspd demo-ui sender lsp recipient; do
  file="$LOG_DIR/$name.pid"
  if [ -f "$file" ]; then
    kill "$(cat "$file")" >/dev/null 2>&1 || true
    echo "[stop] $name"
  fi
done

for node in sender lsp recipient; do
  pkill -f "fnn -c .*runtime/$node/config.yml" >/dev/null 2>&1 || true
  pkill -f "fnn .*runtime/$node/config.yml" >/dev/null 2>&1 || true
done
