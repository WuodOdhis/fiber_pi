#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT_DIR/logs"
FNN_CLI="$ROOT_DIR/.fiber-bin/fnn-cli"
AMOUNT="${1:-10000000000}"
SENDER_URL="${DEMO_SENDER_URL:-http://127.0.0.1:8627}"
LSPD_URL="${DEMO_LSPD_URL:-http://127.0.0.1:3002}"
RECIPIENT_URL="${DEMO_RECIPIENT_URL:-http://127.0.0.1:8827}"
RECIPIENT_P2P_PORT="${DEMO_RECIPIENT_P2P_PORT:-8828}"

mkdir -p "$LOG_DIR"

rpc() {
  local url="$1"
  local method="$2"
  local params="$3"
  curl -sS -H 'content-type: application/json' \
    --data "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" \
    "$url"
}

recipient_pub="$($FNN_CLI -u "$RECIPIENT_URL" -o json --no-banner info | jq -r .pubkey)"

echo "[demo] amount_shannons=$AMOUNT"
echo "[demo] recipient_pubkey=$recipient_pub"

echo "[before] recipient channel balance"
rpc "$RECIPIENT_URL" list_channels '[{"include_closed":false}]' \
  | tee "$LOG_DIR/demo-before-recipient-channels.json" \
  | jq -r '.result.channels[]? | "channel=" + .channel_id + " state=" + .state.state_name + " local=" + .local_balance + " remote=" + .remote_balance'

buy_payload="{\"recipient_pubkey\":\"$recipient_pub\",\"recipient_address\":\"/ip4/127.0.0.1/tcp/$RECIPIENT_P2P_PORT\",\"amount\":\"$AMOUNT\"}"
rpc "$LSPD_URL" buy "$buy_payload" > "$LOG_DIR/demo-buy.json"
order_id="$(jq -r '.result.order_id' "$LOG_DIR/demo-buy.json")"
invoice="$(jq -r '.result.invoice' "$LOG_DIR/demo-buy.json")"
payment_hash="$(jq -r '.result.payment_hash' "$LOG_DIR/demo-buy.json")"
gross="$(jq -r '.result.gross_amount' "$LOG_DIR/demo-buy.json")"
fee="$(jq -r '.result.fee_amount' "$LOG_DIR/demo-buy.json")"
net="$(jq -r '.result.net_amount' "$LOG_DIR/demo-buy.json")"

echo "[buy] order=$order_id gross=$gross fee=$fee net=$net"

send_params="[{\"invoice\":\"$invoice\",\"timeout\":\"0x258\",\"max_fee_amount\":\"0x77359400\"}]"
rpc "$SENDER_URL" send_payment "$send_params" > "$LOG_DIR/demo-send-payment.json"
sender_payment_hash="$(jq -r '.result.payment_hash // empty' "$LOG_DIR/demo-send-payment.json")"
if [ -z "$sender_payment_hash" ]; then
  echo "[error] sender payment failed"
  jq . "$LOG_DIR/demo-send-payment.json"
  exit 1
fi
echo "[sender] payment_hash=$sender_payment_hash"

for i in $(seq 1 180); do
  rpc "$LSPD_URL" get_order_status "{\"order_id\":\"$order_id\"}" > "$LOG_DIR/demo-order-status.json"
  rpc "$SENDER_URL" get_payment "[{\"payment_hash\":\"$payment_hash\"}]" > "$LOG_DIR/demo-payment-status.json"
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

echo "[after] recipient channel balance"
rpc "$RECIPIENT_URL" list_channels '[{"include_closed":false}]' \
  | tee "$LOG_DIR/demo-after-recipient-channels.json" \
  | jq -r '.result.channels[]? | "channel=" + .channel_id + " state=" + .state.state_name + " local=" + .local_balance + " remote=" + .remote_balance'

echo "[events]"
jq -r '.result.events[] | .status + " | " + .reason' "$LOG_DIR/demo-order-status.json"

echo "[result]"
jq -r '.result.status + " | " + .result.status_reason' "$LOG_DIR/demo-order-status.json"
