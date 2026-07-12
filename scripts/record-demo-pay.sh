#!/usr/bin/env bash
set -euo pipefail

AMOUNT="${1:-${DEMO_AMOUNT:-1000000000}}"

DEMO_SENDER_URL=http://127.0.0.1:9227 \
DEMO_LSPD_URL=http://127.0.0.1:3003 \
DEMO_RECIPIENT_URL=http://127.0.0.1:9427 \
DEMO_RECIPIENT_P2P_PORT=9428 \
"$(dirname "$0")/demo-run-payment.sh" "$AMOUNT"
