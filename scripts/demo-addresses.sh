#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CKB_CLI="${CKB_CLI:-$ROOT_DIR/.ckb-cli/ckb-cli_v2.0.0_x86_64-unknown-linux-gnu/ckb-cli}"

if [ ! -x "$CKB_CLI" ]; then
  if command -v ckb-cli >/dev/null 2>&1; then
    CKB_CLI="$(command -v ckb-cli)"
  else
    echo "[error] ckb-cli not found. Set CKB_CLI=/path/to/ckb-cli"
    exit 1
  fi
fi

for node in "${@:-sender lsp recipient}"; do
  key="$ROOT_DIR/runtime/$node/ckb/key"
  if [ ! -f "$key" ]; then
    echo "[error] missing key for $node: $key"
    exit 1
  fi
  info="$($CKB_CLI util key-info --privkey-path "$key" --output-format json)"
  lock_arg="$(jq -r .lock_arg <<<"$info")"
  address="$(jq -r .address.testnet <<<"$info")"
  printf '%s\n  lock_arg: %s\n  testnet_address: %s\n' "$node" "$lock_arg" "$address"
done
