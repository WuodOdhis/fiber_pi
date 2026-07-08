#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NODE_NAME="${1:-}"

if [ -z "$NODE_NAME" ]; then
  echo "usage: scripts/start-node.sh <sender|lsp|recipient>"
  exit 1
fi

NODE_DIR="$ROOT_DIR/runtime/$NODE_NAME"
FNN_BIN="$ROOT_DIR/.fiber-bin/fnn"

if [ ! -x "$FNN_BIN" ]; then
  echo "[error] missing fnn binary: $FNN_BIN"
  echo "[hint] run scripts/prepare-fiber.sh first"
  exit 1
fi

if [ ! -f "$NODE_DIR/config.yml" ]; then
  echo "[error] missing node config: $NODE_DIR/config.yml"
  echo "[hint] run scripts/init-demo-nodes.sh first"
  exit 1
fi

export FIBER_SECRET_KEY_PASSWORD="${FIBER_SECRET_KEY_PASSWORD:-fiber-demo-password}"
export RUST_LOG="${RUST_LOG:-info}"

echo "[start] node=$NODE_NAME dir=$NODE_DIR"
exec "$FNN_BIN" -c "$NODE_DIR/config.yml" -d "$NODE_DIR"
