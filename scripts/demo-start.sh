#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT_DIR/logs"
mkdir -p "$LOG_DIR"

SENDER_NODE="${DEMO_SENDER_NODE:-sender}"
LSP_NODE="${DEMO_LSP_NODE:-lsp}"
RECIPIENT_NODE="${DEMO_RECIPIENT_NODE:-recipient}"
SENDER_RPC_PORT="${DEMO_SENDER_RPC_PORT:-8627}"
LSP_RPC_PORT="${DEMO_LSP_RPC_PORT:-8727}"
RECIPIENT_RPC_PORT="${DEMO_RECIPIENT_RPC_PORT:-8827}"
SENDER_P2P_PORT="${DEMO_SENDER_P2P_PORT:-8628}"
LSP_P2P_PORT="${DEMO_LSP_P2P_PORT:-8728}"
RECIPIENT_P2P_PORT="${DEMO_RECIPIENT_P2P_PORT:-8828}"
LSPD_PORT="${DEMO_LSPD_PORT:-3002}"
UI_PORT="${DEMO_UI_PORT:-5173}"
DEMO_AMOUNT="${DEMO_AMOUNT:-1000000000}"
REQUIRED_OUTBOUND="${DEMO_REQUIRED_OUTBOUND:-$DEMO_AMOUNT}"
SENDER_CHANNEL_AMOUNT="${DEMO_SENDER_CHANNEL_AMOUNT:-20000000000}"

SENDER_URL="http://127.0.0.1:$SENDER_RPC_PORT"
LSP_URL="http://127.0.0.1:$LSP_RPC_PORT"
RECIPIENT_URL="http://127.0.0.1:$RECIPIENT_RPC_PORT"
LSPD_URL="http://127.0.0.1:$LSPD_PORT"

FNN="$ROOT_DIR/.fiber-bin/fnn"
FNN_CLI="$ROOT_DIR/.fiber-bin/fnn-cli"

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

stop_pid_file() {
  local file="$1"
  if [ -f "$file" ]; then
    kill "$(cat "$file")" >/dev/null 2>&1 || true
  fi
}

stop_node_by_name() {
  local node="$1"
  pkill -f "fnn -c .*runtime/$node/config.yml" >/dev/null 2>&1 || true
  pkill -f "fnn .*runtime/$node/config.yml" >/dev/null 2>&1 || true
}

assert_port_free() {
  local port="$1"
  if ss -ltn "sport = :$port" | grep -q LISTEN; then
    ss -ltnp "sport = :$port" || true
    die "port $port is already in use. Stop the process or choose another DEMO_*_PORT."
  fi
}

start_fnn() {
  local node="$1"
  local log="$LOG_DIR/$node.log"
  local pid_file="$LOG_DIR/$node.pid"

  stop_pid_file "$pid_file"
  stop_node_by_name "$node"
  sleep 1

  FIBER_SECRET_KEY_PASSWORD="${FIBER_SECRET_KEY_PASSWORD:-fiber-demo-password}" \
  RUST_LOG="${RUST_LOG:-info}" \
  setsid "$FNN" -c "$ROOT_DIR/runtime/$node/config.yml" -d "$ROOT_DIR/runtime/$node" \
    > "$log" 2>&1 < /dev/null &
  echo $! > "$pid_file"
  echo "[start] $node pid=$(cat "$pid_file") log=$log"
}

wait_rpc() {
  local name="$1"
  local url="$2"
  local log="$3"
  for _ in $(seq 1 60); do
    if "$FNN_CLI" -u "$url" -o json --no-banner info >/dev/null 2>&1; then
      echo "[ready] $name rpc=$url"
      return 0
    fi
    if [ -f "$log" ] && grep -q "Address already in use" "$log"; then
      die "$name failed to start because its configured port is already in use. See $log"
    fi
    sleep 1
  done
  die "$name did not become ready at $url. See $log"
}

connect_peer() {
  local from_name="$1"
  local from_url="$2"
  local address="$3"
  local pubkey="$4"
  "$FNN_CLI" -u "$from_url" -o json --no-banner peer connect_peer \
    --address "$address" \
    --pubkey "$pubkey" \
    --save true >/dev/null 2>&1 || true

  for i in $(seq 1 20); do
    if "$FNN_CLI" -u "$from_url" -o json --no-banner peer list_peers \
      | jq -e --arg pubkey "$pubkey" '.peers[]? | select(.pubkey == $pubkey)' >/dev/null; then
      echo "[connect] $from_name -> $pubkey"
      return 0
    fi
    sleep 1
  done
  die "$from_name could not confirm peer $pubkey"
}

sender_outbound_capacity() {
  local lsp_pub="$1"
  local file="$LOG_DIR/demo-sender-lsp-capacity.json"
  rpc "$SENDER_URL" list_channels "[{\"pubkey\":\"$lsp_pub\",\"include_closed\":false}]" > "$file"

  local max=0
  while read -r state local_balance offered_balance; do
    if [ "$state" != "ChannelReady" ]; then
      continue
    fi
    local local_dec=$((local_balance))
    local offered_dec=$((offered_balance))
    local available=$((local_dec - offered_dec))
    if [ "$available" -gt "$max" ]; then
      max="$available"
    fi
  done < <(jq -r '.result.channels[]? | [.state.state_name, .local_balance, .offered_tlc_balance] | @tsv' "$file")
  echo "$max"
}

open_sender_liquidity_if_needed() {
  local lsp_pub="$1"
  local available
  available="$(sender_outbound_capacity "$lsp_pub")"
  echo "[liquidity] $SENDER_NODE max_outbound=$available required=$REQUIRED_OUTBOUND"
  if [ "$available" -ge "$REQUIRED_OUTBOUND" ]; then
    return 0
  fi

  echo "[liquidity] opening $SENDER_NODE -> $LSP_NODE channel amount=$SENDER_CHANNEL_AMOUNT"
  "$FNN_CLI" -u "$SENDER_URL" -o json --no-banner channel open_channel \
    --pubkey "$lsp_pub" \
    --funding-amount "$SENDER_CHANNEL_AMOUNT" \
    --public true > "$LOG_DIR/demo-open-$SENDER_NODE-$LSP_NODE.json"

  for i in $(seq 1 72); do
    available="$(sender_outbound_capacity "$lsp_pub")"
    echo "[liquidity] poll=$i $SENDER_NODE max_outbound=$available"
    if [ "$available" -ge "$REQUIRED_OUTBOUND" ]; then
      echo "[liquidity] $SENDER_NODE outbound ready"
      return 0
    fi
    sleep 5
  done

  echo "[diagnostic] sender channels:"
  rpc "$SENDER_URL" list_channels '[{"include_closed":true}]' | jq .
  die "$SENDER_NODE outbound liquidity is $available, below required $REQUIRED_OUTBOUND. Use a smaller DEMO_AMOUNT or open a larger sender channel."
}

recipient_open_channel_count() {
  rpc "$RECIPIENT_URL" list_channels '[{"include_closed":false}]' \
    | jq -r '.result.channels | length'
}

start_lspd() {
  stop_pid_file "$LOG_DIR/lspd.pid"
  pkill -f "target/debug/lspd" >/dev/null 2>&1 || true
  assert_port_free "$LSPD_PORT"

  FIBER_RPC_URL="$LSP_URL" \
  LSP_LISTEN_ADDR="127.0.0.1:$LSPD_PORT" \
  POLL_INTERVAL_MS=1000 \
  ORDER_TIMEOUT_SECONDS=7200 \
  RUST_LOG=info \
  setsid cargo run -q -p lspd \
    > "$LOG_DIR/lspd.log" 2>&1 < /dev/null &
  echo $! > "$LOG_DIR/lspd.pid"
  echo "[start] lspd pid=$(cat "$LOG_DIR/lspd.pid") log=$LOG_DIR/lspd.log"

  for _ in $(seq 1 60); do
    if rpc "$LSPD_URL" get_info '{}' | jq -e '.result.service == "fiber-lsp-daemon"' >/dev/null 2>&1; then
      echo "[ready] lspd rpc=$LSPD_URL"
      return 0
    fi
    sleep 1
  done
  die "lspd did not become ready. See $LOG_DIR/lspd.log"
}

start_ui() {
  stop_pid_file "$LOG_DIR/demo-ui.pid"
  pkill -f "demo-ui/server.js" >/dev/null 2>&1 || true
  pkill -f "node server.js" >/dev/null 2>&1 || true
  sleep 1
  assert_port_free "$UI_PORT"

  (
    cd "$ROOT_DIR/demo-ui"
    DEMO_UI_PORT="$UI_PORT" \
    LSPD_URL="$LSPD_URL" \
    SENDER_FIBER_URL="$SENDER_URL" \
    LSP_FIBER_URL="$LSP_URL" \
    RECIPIENT_FIBER_URL="$RECIPIENT_URL" \
    RECIPIENT_PUBKEY="$recipient_pub" \
    RECIPIENT_ADDRESS="/ip4/127.0.0.1/tcp/$RECIPIENT_P2P_PORT" \
    DEMO_AMOUNT="$DEMO_AMOUNT" \
    node server.js > "$LOG_DIR/demo-ui.log" 2>&1 < /dev/null &
    echo $! > "$LOG_DIR/demo-ui.pid"
  )

  for _ in $(seq 1 30); do
    if curl -sS "http://127.0.0.1:$UI_PORT/api/config" >/dev/null 2>&1; then
      echo "[ready] demo-ui url=http://127.0.0.1:$UI_PORT"
      return 0
    fi
    sleep 1
  done
  die "demo-ui did not become ready. See $LOG_DIR/demo-ui.log"
}

[ -x "$FNN" ] || die "missing Fiber node binary; run scripts/prepare-fiber.sh first"
[ -x "$FNN_CLI" ] || die "missing Fiber CLI binary; run scripts/prepare-fiber.sh first"
command -v jq >/dev/null 2>&1 || die "jq is required"
command -v curl >/dev/null 2>&1 || die "curl is required"

for node in "$SENDER_NODE" "$LSP_NODE" "$RECIPIENT_NODE"; do
  [ -f "$ROOT_DIR/runtime/$node/config.yml" ] || die "missing runtime/$node/config.yml; run scripts/init-demo-nodes.sh first"
done

stop_pid_file "$LOG_DIR/lspd.pid"
stop_pid_file "$LOG_DIR/demo-ui.pid"
stop_node_by_name "$SENDER_NODE"
stop_node_by_name "$LSP_NODE"
stop_node_by_name "$RECIPIENT_NODE"
sleep 2

assert_port_free "$SENDER_RPC_PORT"
assert_port_free "$SENDER_P2P_PORT"
assert_port_free "$LSP_RPC_PORT"
assert_port_free "$LSP_P2P_PORT"
assert_port_free "$RECIPIENT_RPC_PORT"
assert_port_free "$RECIPIENT_P2P_PORT"

start_fnn "$SENDER_NODE"
start_fnn "$LSP_NODE"
start_fnn "$RECIPIENT_NODE"

wait_rpc "$SENDER_NODE" "$SENDER_URL" "$LOG_DIR/$SENDER_NODE.log"
wait_rpc "$LSP_NODE" "$LSP_URL" "$LOG_DIR/$LSP_NODE.log"
wait_rpc "$RECIPIENT_NODE" "$RECIPIENT_URL" "$LOG_DIR/$RECIPIENT_NODE.log"

sender_pub="$($FNN_CLI -u "$SENDER_URL" -o json --no-banner info | jq -r .pubkey)"
lsp_pub="$($FNN_CLI -u "$LSP_URL" -o json --no-banner info | jq -r .pubkey)"
recipient_pub="$($FNN_CLI -u "$RECIPIENT_URL" -o json --no-banner info | jq -r .pubkey)"

echo "[node] $SENDER_NODE=$sender_pub"
echo "[node] $LSP_NODE=$lsp_pub"
echo "[node] $RECIPIENT_NODE=$recipient_pub"

connect_peer "$SENDER_NODE" "$SENDER_URL" "/ip4/127.0.0.1/tcp/$LSP_P2P_PORT" "$lsp_pub"
connect_peer "$LSP_NODE" "$LSP_URL" "/ip4/127.0.0.1/tcp/$RECIPIENT_P2P_PORT" "$recipient_pub"

open_sender_liquidity_if_needed "$lsp_pub"

recipient_channels="$(recipient_open_channel_count)"
echo "[preflight] recipient open channels=$recipient_channels"
if [ "$recipient_channels" != "0" ]; then
  echo "[warn] recipient is not in zero-channel state. For a zero-channel recording, use a fresh recipient runtime."
fi

start_lspd
start_ui

cat <<EOF
[done] demo stack is ready

Use this payment command for the recording:

DEMO_SENDER_URL=$SENDER_URL \\
DEMO_LSPD_URL=$LSPD_URL \\
DEMO_RECIPIENT_URL=$RECIPIENT_URL \\
DEMO_RECIPIENT_P2P_PORT=$RECIPIENT_P2P_PORT \\
scripts/demo-run-payment.sh $DEMO_AMOUNT
EOF
