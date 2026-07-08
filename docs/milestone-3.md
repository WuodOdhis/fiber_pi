# Milestone 3: LSP API

Milestone 3 adds the client-facing LSP API on top of the Fiber RPC client.

The daemon now exposes three JSON-RPC methods:

```text
get_info
buy
get_order_status
```

## Runtime

Start the LSP daemon against the local LSP Fiber node:

```bash
FIBER_RPC_URL=http://127.0.0.1:8427 \
LSP_LISTEN_ADDR=127.0.0.1:3001 \
cargo run -p lspd
```

Default settings:

```text
FIBER_RPC_URL=http://127.0.0.1:8427
LSP_LISTEN_ADDR=127.0.0.1:3001
FEE_RATE_BPS=100
MIN_AMOUNT=100000000
MAX_AMOUNT=10000000000000
FIBER_CURRENCY=Fibt
INVOICE_EXPIRY_SECONDS=3600
```

## `get_info`

Request:

```json
{"jsonrpc":"2.0","method":"get_info","params":{},"id":1}
```

Observed result:

```json
{
  "service": "fiber-lsp-daemon",
  "version": "0.1.0",
  "lsp_pubkey": "0358115f048eefb31d99becefc1d07f3c249faac0dcefd24c70055b5fbd411e08d",
  "fiber_version": "0.8.1",
  "fiber_commit_hash": "b560023 2026-04-16",
  "currency": "Fibt",
  "fee_rate_bps": "100",
  "min_amount": "100000000",
  "max_amount": "10000000000000",
  "invoice_expiry_seconds": 3600
}
```

## `buy`

Request:

```json
{
  "jsonrpc": "2.0",
  "method": "buy",
  "params": {
    "recipient_pubkey": "02913dcce1e35f5ffc32ff04a6ddafccf660f5e88293b59c3c80b6dddf7a0a8406",
    "recipient_address": "/ip4/127.0.0.1/tcp/8528",
    "amount": "100000000000"
  },
  "id": 2
}
```

Observed result shape:

```json
{
  "order_id": "<uuid>",
  "invoice": "fibt1000000000001p...",
  "payment_hash": "0x...",
  "gross_amount": "100000000000",
  "fee_amount": "1000000000",
  "net_amount": "99000000000",
  "currency": "Fibt",
  "status": "AWAITING_PAYMENT",
  "fiber_invoice_status": null
}
```

What `buy` does now:

1. Validates the requested amount against configured min/max.
2. Calculates fee using basis points.
3. Generates a random 32-byte preimage.
4. Computes a SHA-256 payment hash.
5. Creates a real Fiber hold invoice with `hash_algorithm = sha256`.
6. Stores an in-memory order.
7. Returns invoice, order id, fee, net amount, and status.

## `get_order_status`

Request:

```json
{
  "jsonrpc": "2.0",
  "method": "get_order_status",
  "params": {"order_id": "<uuid>"},
  "id": 3
}
```

Observed result shape:

```json
{
  "order_id": "<uuid>",
  "status": "AWAITING_PAYMENT",
  "invoice_status": "Open",
  "payment_hash": "0x...",
  "gross_amount": "100000000000",
  "fee_amount": "1000000000",
  "net_amount": "99000000000",
  "currency": "Fibt"
}
```

`get_order_status` reads the in-memory order and checks the live Fiber invoice state using `get_invoice`.

## Verified Smoke Test

Commands were tested against a running local LSP Fiber node at `http://127.0.0.1:8427`.

Observed terminal summary:

```text
fiber-lsp-daemon 0358115f048eefb31d99becefc1d07f3c249faac0dcefd24c70055b5fbd411e08d
AWAITING_PAYMENT fee=1000000000 net=99000000000
AWAITING_PAYMENT invoice=Open
```

## Not Included Yet

This milestone does not include watchers or automatic progression beyond `AWAITING_PAYMENT`.

Next milestones will add:

```text
invoice watcher
channel watcher
state machine transitions
channel opening after payment detection
settlement after ChannelReady
```
