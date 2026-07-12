#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FNN_BIN="$ROOT_DIR/.fiber-bin/fnn"
FNN_CLI="$ROOT_DIR/.fiber-bin/fnn-cli"

if [ ! -x "$FNN_BIN" ] || [ ! -x "$FNN_CLI" ]; then
  echo "[error] missing Fiber binaries; run scripts/prepare-fiber.sh first"
  exit 1
fi

for node in sender lsp recipient; do
  if [ ! -f "$ROOT_DIR/runtime/$node/config.yml" ]; then
    echo "[error] missing runtime for $node; run scripts/init-demo-nodes.sh first"
    exit 1
  fi
done

mkdir -p "$ROOT_DIR/logs"

pids=""

cleanup() {
  for pid in $pids; do
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
  done
}

trap cleanup EXIT

start_node() {
  local node="$1"
  local log="$ROOT_DIR/logs/$node.log"
  echo "[probe] starting $node"
  FIBER_SECRET_KEY_PASSWORD="${FIBER_SECRET_KEY_PASSWORD:-fiber-demo-password}" \
  RUST_LOG="${RUST_LOG:-info}" \
  "$FNN_BIN" -c "$ROOT_DIR/runtime/$node/config.yml" -d "$ROOT_DIR/runtime/$node" > "$log" 2>&1 &
  pids="$pids $!"
}

start_node sender
start_node lsp
start_node recipient

echo "[probe] waiting for RPC services"
sleep 12

echo "[probe] querying sender"
"$FNN_CLI" -u http://127.0.0.1:8627 -o json --no-banner info

echo "[probe] querying lsp"
"$FNN_CLI" -u http://127.0.0.1:8727 -o json --no-banner info

echo "[probe] querying recipient"
"$FNN_CLI" -u http://127.0.0.1:8827 -o json --no-banner info

echo "[probe] all node RPC endpoints responded"
