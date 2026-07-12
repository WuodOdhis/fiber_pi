#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT_DIR/logs"
FNN_CLI="$ROOT_DIR/.fiber-bin/fnn-cli"
AMOUNT="${1:-${DEMO_AMOUNT:-1000000000}}"
SENDER_URL="${DEMO_SENDER_URL:-http://127.0.0.1:8627}"
LSPD_URL="${DEMO_LSPD_URL:-http://127.0.0.1:3002}"
RECIPIENT_URL="${DEMO_RECIPIENT_URL:-http://127.0.0.1:8827}"
RECIPIENT_P2P_PORT="${DEMO_RECIPIENT_P2P_PORT:-8828}"

mkdir -p "$LOG_DIR"

die() {
  echo "[error] $*" >&2
  exit 1
}

rpc() {
  local url="$1"
  local method="$2"
  local params="$3"
  curl -sS -H 'content-type: application/json' \
    --data "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" \
    "$url"
}

require_rpc() {
  local name="$1"
  local url="$2"
  rpc "$url" node_info '[]' >/dev/null || die "$name RPC is not reachable at $url"
}

case "$AMOUNT" in
  ''|*[!0-9]*) die "amount must be an integer number of shannons, got '$AMOUNT'" ;;
esac

[ -x "$FNN_CLI" ] || die "missing Fiber CLI binary; run scripts/prepare-fiber.sh first"
command -v jq >/dev/null 2>&1 || die "jq is required"
command -v curl >/dev/null 2>&1 || die "curl is required"

require_rpc "sender" "$SENDER_URL"
require_rpc "recipient" "$RECIPIENT_URL"
rpc "$LSPD_URL" get_info '{}' | jq -e '.result.service == "fiber-lsp-daemon"' >/dev/null \
  || die "lspd is not reachable at $LSPD_URL"

recipient_pub="$($FNN_CLI -u "$RECIPIENT_URL" -o json --no-banner info | jq -r .pubkey)"

echo "[demo] amount_shannons=$AMOUNT"
echo "[demo] sender_rpc=$SENDER_URL"
echo "[demo] lspd_rpc=$LSPD_URL"
echo "[demo] recipient_rpc=$RECIPIENT_URL"
echo "[demo] recipient_pubkey=$recipient_pub"

echo "[before] recipient channels"
rpc "$RECIPIENT_URL" list_channels '[{"include_closed":false}]' \
  | tee "$LOG_DIR/demo-before-recipient-channels.json" \
  | jq .

buy_payload="{\"recipient_pubkey\":\"$recipient_pub\",\"recipient_address\":\"/ip4/127.0.0.1/tcp/$RECIPIENT_P2P_PORT\",\"amount\":\"$AMOUNT\"}"
rpc "$LSPD_URL" buy "$buy_payload" > "$LOG_DIR/demo-buy.json"
if jq -e '.error' "$LOG_DIR/demo-buy.json" >/dev/null; then
  jq . "$LOG_DIR/demo-buy.json"
  die "buy failed"
fi

order_id="$(jq -r '.result.order_id' "$LOG_DIR/demo-buy.json")"
invoice="$(jq -r '.result.invoice' "$LOG_DIR/demo-buy.json")"
payment_hash="$(jq -r '.result.payment_hash' "$LOG_DIR/demo-buy.json")"
gross="$(jq -r '.result.gross_amount' "$LOG_DIR/demo-buy.json")"
fee="$(jq -r '.result.fee_amount' "$LOG_DIR/demo-buy.json")"
net="$(jq -r '.result.net_amount' "$LOG_DIR/demo-buy.json")"

echo "[buy] order=$order_id gross=$gross fee=$fee net=$net"
echo "[buy] payment_hash=$payment_hash"

send_params="[{\"invoice\":\"$invoice\",\"timeout\":\"0x258\",\"max_fee_amount\":\"0x77359400\"}]"
rpc "$SENDER_URL" send_payment "$send_params" > "$LOG_DIR/demo-send-payment.json"
if jq -e '.error' "$LOG_DIR/demo-send-payment.json" >/dev/null; then
  jq . "$LOG_DIR/demo-send-payment.json"
  die "sender send_payment failed"
fi

sender_payment_hash="$(jq -r '.result.payment_hash // empty' "$LOG_DIR/demo-send-payment.json")"
[ -n "$sender_payment_hash" ] || die "send_payment response did not include payment_hash"
echo "[sender] payment_hash=$sender_payment_hash"

for i in $(seq 1 180); do
  rpc "$LSPD_URL" get_order_status "{\"order_id\":\"$order_id\"}" > "$LOG_DIR/demo-order-status.json"
  rpc "$SENDER_URL" get_payment "[{\"payment_hash\":\"$sender_payment_hash\"}]" > "$LOG_DIR/demo-payment-status.json"

  order_status="$(jq -r '.result.status // .error.message' "$LOG_DIR/demo-order-status.json")"
  invoice_status="$(jq -r '.result.invoice_status // "null"' "$LOG_DIR/demo-order-status.json")"
  payment_status="$(jq -r '.result.status // .error.message' "$LOG_DIR/demo-payment-status.json")"
  reason="$(jq -r '.result.status_reason // ""' "$LOG_DIR/demo-order-status.json")"
  echo "[poll $i] order=$order_status invoice=$invoice_status sender_payment=$payment_status reason=$reason"

  if [ "$order_status" = "COMPLETED" ] || [ "$order_status" = "FAILED" ] || [ "$payment_status" = "Failed" ]; then
    break
  fi
  sleep 2
done

echo "[after] recipient channels"
rpc "$RECIPIENT_URL" list_channels '[{"include_closed":false}]' \
  | tee "$LOG_DIR/demo-after-recipient-channels.json" \
  | jq .

echo "[events]"
jq -r '.result.events[]? | .status + " | " + .reason' "$LOG_DIR/demo-order-status.json"

echo "[result]"
jq -r '.result.status + " | " + .result.status_reason' "$LOG_DIR/demo-order-status.json"

if [ "$(jq -r '.result.status // empty' "$LOG_DIR/demo-order-status.json")" != "COMPLETED" ]; then
  echo "[diagnostic] order status:"
  jq . "$LOG_DIR/demo-order-status.json"
  echo "[diagnostic] sender payment status:"
  jq . "$LOG_DIR/demo-payment-status.json"
  exit 1
fi

cat <<EOF
[proof]
order_id=$order_id
payment_hash=$sender_payment_hash
order_status=COMPLETED

Screenshot the final recipient channel JSON above. The channel_outpoint contains the funding tx hash.
EOF
