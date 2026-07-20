#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CKB_CLI_DIR="$ROOT_DIR/.ckb-cli"
CKB_CLI="$CKB_CLI_DIR/ckb-cli_v2.0.0_x86_64-unknown-linux-gnu/ckb-cli"

export DEMO_SENDER_NODE="${DEMO_SENDER_NODE:-railway-sender}"
export DEMO_LSP_NODE="${DEMO_LSP_NODE:-railway-lsp}"
export DEMO_RECIPIENT_NODE="${DEMO_RECIPIENT_NODE:-railway-recipient}"
export DEMO_UI_HOST="${DEMO_UI_HOST:-0.0.0.0}"
export DEMO_LSPD_PORT="${DEMO_LSPD_PORT:-3002}"
export DEMO_AMOUNT="${DEMO_AMOUNT:-1000000000}"

if [ -n "${PORT:-}" ] && [ -z "${DEMO_UI_PORT:-}" ]; then
  export DEMO_UI_PORT="$PORT"
fi

install_ckb_cli_if_missing() {
  if [ -x "$CKB_CLI" ]; then
    return 0
  fi

  mkdir -p "$CKB_CLI_DIR" /tmp/fiber-lsp-railway
  echo "[railway] installing ckb-cli for funding address output"
  curl -L \
    https://github.com/nervosnetwork/ckb-cli/releases/download/v2.0.0/ckb-cli_v2.0.0_x86_64-unknown-linux-gnu.tar.gz \
    -o /tmp/fiber-lsp-railway/ckb-cli.tar.gz
  tar -xzf /tmp/fiber-lsp-railway/ckb-cli.tar.gz -C "$CKB_CLI_DIR"
  chmod +x "$CKB_CLI"
}

if [ ! -x "$ROOT_DIR/.fiber-bin/fnn" ] || [ ! -x "$ROOT_DIR/.fiber-bin/fnn-cli" ]; then
  echo "[railway] Fiber binaries missing; building Fiber"
  "$ROOT_DIR/scripts/prepare-fiber.sh"
fi

if [ ! -f "$ROOT_DIR/runtime/$DEMO_SENDER_NODE/config.yml" ] \
  || [ ! -f "$ROOT_DIR/runtime/$DEMO_LSP_NODE/config.yml" ] \
  || [ ! -f "$ROOT_DIR/runtime/$DEMO_RECIPIENT_NODE/config.yml" ]; then
  echo "[railway] initializing demo nodes"
  "$ROOT_DIR/scripts/init-demo-nodes.sh"
fi

install_ckb_cli_if_missing

cat <<'EOF'
[railway] funding address output:
EOF
if ! CKB_CLI="$CKB_CLI" "$ROOT_DIR/scripts/demo-addresses.sh" \
  "$DEMO_SENDER_NODE" "$DEMO_LSP_NODE" "$DEMO_RECIPIENT_NODE"; then
  cat <<'EOF'
[railway] warning: could not print funding addresses with ckb-cli.
[railway] This can happen because Fiber stores encrypted node keys, while ckb-cli expects plaintext hex keys.
[railway] If these Railway node addresses were already funded, this warning is safe and startup will continue.
EOF
fi

cat <<EOF
[railway] suggested testnet funding:
  $DEMO_SENDER_NODE: 500 CKB+
  $DEMO_LSP_NODE: 500 CKB+
  $DEMO_RECIPIENT_NODE: 221 CKB
[railway] after funding, redeploy or restart this Railway service.
EOF

exec "$ROOT_DIR/scripts/demo-start.sh"
