#!/usr/bin/env bash
set -euo pipefail

AMOUNT="${1:-${DEMO_AMOUNT:-1000000000}}"

DEMO_SENDER_URL=http://127.0.0.1:8627 \
DEMO_LSPD_URL=http://127.0.0.1:3002 \
DEMO_RECIPIENT_URL=http://127.0.0.1:8827 \
DEMO_RECIPIENT_P2P_PORT=8828 \
"$(dirname "$0")/demo-run-payment.sh" "$AMOUNT"
