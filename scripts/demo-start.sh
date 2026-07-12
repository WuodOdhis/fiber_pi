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

SENDER_URL="http://127.0.0.1:$SENDER_RPC_PORT"
LSP_URL="http://127.0.0.1:$LSP_RPC_PORT"
RECIPIENT_URL="http://127.0.0.1:$RECIPIENT_RPC_PORT"
LSPD_URL="http://127.0.0.1:$LSPD_PORT"

FNN="$ROOT_DIR/.fiber-bin/fnn"
FNN_CLI="$ROOT_DIR/.fiber-bin/fnn-cli"

if [ ! -x "$FNN" ] || [ ! -x "$FNN_CLI" ]; then
  echo "[error] missing Fiber binaries; run scripts/prepare-fiber.sh first"
  exit 1
fi

for node in "$SENDER_NODE" "$LSP_NODE" "$RECIPIENT_NODE"; do
  if [ ! -f "$ROOT_DIR/runtime/$node/config.yml" ]; then
    echo "[error] missing runtime/$node/config.yml"
    echo "[hint] this demo uses the funded clean runtime directories"
    exit 1
  fi
done

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
  for _ in $(seq 1 60); do
    if "$FNN_CLI" -u "$url" -o json --no-banner info >/dev/null 2>&1; then
      echo "[ready] $name rpc=$url"
      return 0
    fi
    sleep 1
  done
  echo "[error] $name did not become ready at $url"
  exit 1
}

sender_outbound_capacity() {
  local lsp_pub="$1"
  local file="$LOG_DIR/demo-sender-lsp-capacity.json"
  curl -sS -H 'content-type: application/json' \
    --data "{\"jsonrpc\":\"2.0\",\"method\":\"list_channels\",\"params\":[{\"pubkey\":\"$lsp_pub\",\"include_closed\":false}],\"id\":1}" \
    "$SENDER_URL" > "$file"

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

ensure_sender_liquidity() {
  local lsp_pub="$1"
  local required="${DEMO_REQUIRED_OUTBOUND:-10000000000}"
  local channel_amount="${DEMO_SENDER_CHANNEL_AMOUNT:-80000000000}"
  local available
  available="$(sender_outbound_capacity "$lsp_pub")"
  echo "[liquidity] $SENDER_NODE max_outbound=$available required=$required"
  if [ "$available" -ge "$required" ]; then
    return 0
  fi

  local channel_hex
  channel_hex="$(printf '0x%x' "$channel_amount")"
  echo "[liquidity] opening $SENDER_NODE -> $LSP_NODE channel amount=$channel_amount"
  for i in $(seq 1 30); do
    curl -sS -H 'content-type: application/json' \
      --data "{\"jsonrpc\":\"2.0\",\"method\":\"open_channel\",\"params\":[{\"pubkey\":\"$lsp_pub\",\"funding_amount\":\"$channel_hex\",\"public\":true}],\"id\":1}" \
      "$SENDER_URL" > "$LOG_DIR/demo-open-$SENDER_NODE-$LSP_NODE.json"
    if ! jq -e '.error' "$LOG_DIR/demo-open-$SENDER_NODE-$LSP_NODE.json" >/dev/null; then
      break
    fi
    message="$(jq -r '.error.message' "$LOG_DIR/demo-open-$SENDER_NODE-$LSP_NODE.json")"
    echo "[liquidity] open retry=$i reason=$message"
    sleep 2
  done

  if jq -e '.error' "$LOG_DIR/demo-open-$SENDER_NODE-$LSP_NODE.json" >/dev/null; then
    jq . "$LOG_DIR/demo-open-$SENDER_NODE-$LSP_NODE.json"
    exit 1
  fi

  for i in $(seq 1 120); do
    available="$(sender_outbound_capacity "$lsp_pub")"
    echo "[liquidity] poll=$i $SENDER_NODE max_outbound=$available"
    if [ "$available" -ge "$required" ]; then
      echo "[liquidity] $SENDER_NODE outbound ready"
      return 0
    fi
    sleep 10
  done

  echo "[error] $SENDER_NODE outbound liquidity did not become ready"
  exit 1
}

start_fnn "$SENDER_NODE"
start_fnn "$LSP_NODE"
start_fnn "$RECIPIENT_NODE"

wait_rpc "$SENDER_NODE" "$SENDER_URL"
wait_rpc "$LSP_NODE" "$LSP_URL"
wait_rpc "$RECIPIENT_NODE" "$RECIPIENT_URL"

sender_pub="$($FNN_CLI -u "$SENDER_URL" -o json --no-banner info | jq -r .pubkey)"
lsp_pub="$($FNN_CLI -u "$LSP_URL" -o json --no-banner info | jq -r .pubkey)"
recipient_pub="$($FNN_CLI -u "$RECIPIENT_URL" -o json --no-banner info | jq -r .pubkey)"

echo "[node] $SENDER_NODE=$sender_pub"
echo "[node] $LSP_NODE=$lsp_pub"
echo "[node] $RECIPIENT_NODE=$recipient_pub"

"$FNN_CLI" -u "$SENDER_URL" -o json --no-banner peer connect_peer \
  --address "/ip4/127.0.0.1/tcp/$LSP_P2P_PORT" \
  --pubkey "$lsp_pub" \
  --save true >/dev/null

"$FNN_CLI" -u "$LSP_URL" -o json --no-banner peer connect_peer \
  --address "/ip4/127.0.0.1/tcp/$RECIPIENT_P2P_PORT" \
  --pubkey "$recipient_pub" \
  --save true >/dev/null

echo "[connect] $SENDER_NODE -> $LSP_NODE"
echo "[connect] $LSP_NODE -> $RECIPIENT_NODE"

ensure_sender_liquidity "$lsp_pub"

if [ -f "$LOG_DIR/lspd.pid" ]; then
  stop_pid_file "$LOG_DIR/lspd.pid"
fi
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
  if curl -sS -H 'content-type: application/json' \
    --data '{"jsonrpc":"2.0","method":"get_info","params":{},"id":1}' \
    "$LSPD_URL" | jq -e '.result.service == "fiber-lsp-daemon"' >/dev/null 2>&1; then
    echo "[ready] lspd rpc=$LSPD_URL"
    break
  fi
  sleep 1
done

if [ -f "$LOG_DIR/demo-ui.pid" ]; then
  stop_pid_file "$LOG_DIR/demo-ui.pid"
fi
(
  cd "$ROOT_DIR/demo-ui"
  DEMO_UI_PORT="$UI_PORT" \
  LSPD_URL="$LSPD_URL" \
  SENDER_FIBER_URL="$SENDER_URL" \
  LSP_FIBER_URL="$LSP_URL" \
  RECIPIENT_FIBER_URL="$RECIPIENT_URL" \
  RECIPIENT_PUBKEY="$recipient_pub" \
  RECIPIENT_ADDRESS="/ip4/127.0.0.1/tcp/$RECIPIENT_P2P_PORT" \
  node server.js > "$LOG_DIR/demo-ui.log" 2>&1 < /dev/null &
  echo $! > "$LOG_DIR/demo-ui.pid"
)
echo "[start] demo-ui pid=$(cat "$LOG_DIR/demo-ui.pid") url=http://127.0.0.1:$UI_PORT"

echo "[done] demo stack is ready"
