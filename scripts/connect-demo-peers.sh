#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FNN_CLI="$ROOT_DIR/.fiber-bin/fnn-cli"

if [ ! -x "$FNN_CLI" ]; then
  echo "[error] missing fnn-cli binary; run scripts/prepare-fiber.sh first"
  exit 1
fi

sender_url="http://127.0.0.1:8327"
lsp_url="http://127.0.0.1:8427"
recipient_url="http://127.0.0.1:8527"

lsp_pub="$($FNN_CLI -u "$lsp_url" -o json --no-banner info | jq -r .pubkey)"
recipient_pub="$($FNN_CLI -u "$recipient_url" -o json --no-banner info | jq -r .pubkey)"

echo "[connect] sender -> lsp ($lsp_pub)"
"$FNN_CLI" -u "$sender_url" -o json --no-banner peer connect_peer \
  --address /ip4/127.0.0.1/tcp/8428 \
  --pubkey "$lsp_pub" \
  --save true

echo "[connect] lsp -> recipient ($recipient_pub)"
"$FNN_CLI" -u "$lsp_url" -o json --no-banner peer connect_peer \
  --address /ip4/127.0.0.1/tcp/8528 \
  --pubkey "$recipient_pub" \
  --save true

echo "[connect] sender peers"
"$FNN_CLI" -u "$sender_url" -o json --no-banner peer list_peers

echo "[connect] lsp peers"
"$FNN_CLI" -u "$lsp_url" -o json --no-banner peer list_peers

echo "[connect] recipient peers"
"$FNN_CLI" -u "$recipient_url" -o json --no-banner peer list_peers
