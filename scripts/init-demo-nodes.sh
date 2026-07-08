#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIBER_VERSION="v0.8.1"
SRC_CONFIG="$ROOT_DIR/.fiber-src/fiber-$FIBER_VERSION/config/testnet/config.yml"
RUNTIME_DIR="$ROOT_DIR/runtime"

if [ ! -f "$SRC_CONFIG" ]; then
  echo "[error] missing Fiber config: $SRC_CONFIG"
  echo "[hint] run scripts/prepare-fiber.sh first"
  exit 1
fi

make_node() {
  local name="$1"
  local p2p_port="$2"
  local rpc_port="$3"
  local node_dir="$RUNTIME_DIR/$name"

  mkdir -p "$node_dir/ckb" "$node_dir/fiber" "$ROOT_DIR/logs"
  cp "$SRC_CONFIG" "$node_dir/config.yml"

  perl -0pi -e "s#listening_addr: \"/ip4/0\.0\.0\.0/tcp/8228\"#listening_addr: \"/ip4/127.0.0.1/tcp/$p2p_port\"#" "$node_dir/config.yml"
  perl -0pi -e "s#listening_addr: \"127\.0\.0\.1:8227\"#listening_addr: \"127.0.0.1:$rpc_port\"#" "$node_dir/config.yml"
  perl -0pi -e 's#bootnode_addrs:\n    - "/ip4/54\.179\.226\.154/tcp/8228/p2p/Qmes1EBD4yNo9Ywkfe6eRw9tG1nVNGLDmMud1xJMsoYFKy"\n    - "/ip4/16\.163\.7\.105/tcp/8228/p2p/QmdyQWjPtbK4NWWsvy8s69NGJaQULwgeQDT5ZpNDrTNaeV"#bootnode_addrs: []#' "$node_dir/config.yml"
  perl -0pi -e "s#announce_listening_addr: true#announce_listening_addr: false#" "$node_dir/config.yml"

  if [ ! -f "$node_dir/ckb/key" ]; then
    if command -v openssl >/dev/null 2>&1; then
      openssl rand -hex 32 > "$node_dir/ckb/key"
    else
      echo "[error] openssl is required to generate demo private keys"
      exit 1
    fi
  fi

  chmod 600 "$node_dir/ckb/key"
  echo "[init] $name -> p2p=$p2p_port rpc=$rpc_port dir=$node_dir"
}

make_node sender 8328 8327
make_node lsp 8428 8427
make_node recipient 8528 8527

echo "[init] demo nodes initialized under $RUNTIME_DIR"
