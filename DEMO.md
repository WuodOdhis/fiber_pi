# Demo Runbook

This runbook describes how to run a clean receive-first payment demo on CKB testnet.

The demo uses three local Fiber nodes:

```text
sender     pays the LSP invoice
lsp        runs the funded LSP Fiber node used by lspd
recipient  starts without a Fiber channel and receives the net payment
```

The goal is to demonstrate the following sequence:

1. The recipient starts with zero open Fiber channels.
2. The sender pays a Fiber invoice issued by the LSP.
3. The LSP provisions recipient-side liquidity by opening a private one-way Fiber channel when needed.
4. The LSP pays the recipient over Fiber.
5. The LSP settles the sender invoice only after the recipient payment succeeds.
6. The recipient ends with non-zero Fiber `local_balance` in a channel it did not pre-create.

## Clean Runtime

Use fresh runtime directories for a clean first-receive demonstration. Failed or interrupted Fiber payments can leave inflight TLCs that temporarily consume channel liquidity.

Create a named demo node set:

```bash
DEMO_SENDER_NODE=demo-sender \
DEMO_LSP_NODE=demo-lsp \
DEMO_RECIPIENT_NODE=demo-recipient \
DEMO_SENDER_RPC_PORT=8927 \
DEMO_SENDER_P2P_PORT=8928 \
DEMO_LSP_RPC_PORT=9027 \
DEMO_LSP_P2P_PORT=9028 \
DEMO_RECIPIENT_RPC_PORT=9127 \
DEMO_RECIPIENT_P2P_PORT=9128 \
scripts/init-demo-nodes.sh
```

Print the generated CKB testnet addresses:

```bash
scripts/demo-addresses.sh demo-sender demo-lsp demo-recipient
```

Suggested funding for a small `10 CKB` testnet payment:

```text
demo-sender:    500 CKB or more
demo-lsp:       500 CKB or more
demo-recipient: 221 CKB
```

For larger payments, fund sender and LSP more generously. Recipient funding is not inbound liquidity; it is a CKB reserve needed by current Fiber/CKB channel acceptor mechanics.

## Start The Stack

Start the three Fiber nodes, `lspd`, and the demo UI:

```bash
DEMO_SENDER_NODE=demo-sender \
DEMO_LSP_NODE=demo-lsp \
DEMO_RECIPIENT_NODE=demo-recipient \
DEMO_SENDER_RPC_PORT=8927 \
DEMO_SENDER_P2P_PORT=8928 \
DEMO_LSP_RPC_PORT=9027 \
DEMO_LSP_P2P_PORT=9028 \
DEMO_RECIPIENT_RPC_PORT=9127 \
DEMO_RECIPIENT_P2P_PORT=9128 \
DEMO_LSPD_PORT=3003 \
DEMO_UI_PORT=5174 \
DEMO_AMOUNT=1000000000 \
scripts/demo-start.sh
```

The script performs preflight checks:

- starts the configured Fiber nodes;
- connects sender to LSP and LSP to recipient;
- ensures sender outbound liquidity for the configured demo amount;
- reports the recipient open-channel count;
- starts `lspd`;
- starts the browser UI.

Open the UI:

```text
http://127.0.0.1:5174
```

## Verify Initial Recipient State

Before running a payment, the recipient should have no open channels:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"list_channels","params":[{"include_closed":false}],"id":1}' \
  http://127.0.0.1:9127 | jq .
```

Expected result:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "channels": []
  }
}
```

## Run A Payment

Run a `10 CKB` payment:

```bash
DEMO_SENDER_URL=http://127.0.0.1:8927 \
DEMO_LSPD_URL=http://127.0.0.1:3003 \
DEMO_RECIPIENT_URL=http://127.0.0.1:9127 \
DEMO_RECIPIENT_P2P_PORT=9128 \
scripts/demo-run-payment.sh 1000000000
```

`1000000000` shannons is `10 CKB`.

Successful completion prints:

```text
COMPLETED | recipient paid ...; Fiber invoice settled; LSP fee earned ...
```

The same flow can also be run from the demo UI.

## Verify Final State

Query the recipient after completion:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"list_channels","params":[{"include_closed":false}],"id":1}' \
  http://127.0.0.1:9127 | jq .
```

Important fields:

```text
state.state_name: ChannelReady
is_acceptor: true
is_one_way: true
local_balance: non-zero recipient balance
channel_outpoint: on-chain funding outpoint
```

Interpretation:

- `is_acceptor: true` means the recipient accepted the channel.
- The LSP side of the same channel has `is_acceptor: false`, showing the LSP opened/funded the channel.
- `local_balance` on the recipient side is the Fiber balance received.
- `channel_outpoint` points to the CKB funding transaction for the channel.

## Channel Funding Transaction

The recipient channel response includes a `channel_outpoint`:

```text
0x5a0964d46f1620af6e5ea590ae304583a9f6eb6a936fa9c57b28434917b054ad00000000
```

Split it as:

```text
funding tx hash: 0x5a0964d46f1620af6e5ea590ae304583a9f6eb6a936fa9c57b28434917b054ad
output index:     00000000
```

The funding transaction proves that a Fiber channel was opened on CKB. The payment itself is off-chain, so its proof comes from Fiber RPC state:

```text
sender get_payment(payment_hash) => Success
get_order_status(order_id) => COMPLETED
invoice_status => Paid
recipient local_balance increased
```

## Useful Verification Commands

Check LSP order status:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"get_order_status","params":{"order_id":"ORDER_ID"},"id":1}' \
  http://127.0.0.1:3003 | jq .
```

Check sender payment status:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"get_payment","params":[{"payment_hash":"PAYMENT_HASH"}],"id":1}' \
  http://127.0.0.1:8927 | jq .
```

Check the LSP-side channel view:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"list_channels","params":[{"include_closed":false}],"id":1}' \
  http://127.0.0.1:9027 | jq .
```

## Cleanup

Stop the configured stack:

```bash
DEMO_SENDER_NODE=demo-sender \
DEMO_LSP_NODE=demo-lsp \
DEMO_RECIPIENT_NODE=demo-recipient \
scripts/demo-stop.sh
```

For another clean first-receive run, create a new runtime node set with fresh names and ports.
