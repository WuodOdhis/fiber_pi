# Demo Guide

This guide is for recording or screenshotting the receive-first payment demo from a clean state.

Use the terminal as the source of truth and the browser UI as the visual layer. The terminal gives the clearest proof. The UI gives a readable screenshot of the order lifecycle.

## Goal

Show this sequence:

1. The recipient starts with zero Fiber channels.
2. The sender pays an LSP invoice.
3. The LSP opens/provisions recipient-side liquidity.
4. The LSP pays the recipient over Fiber.
5. The sender invoice is settled only after the recipient payment succeeds.
6. The recipient ends with Fiber local balance in a channel it did not pre-create.

## Clean Recording Nodes

For a clean recording, do not reuse old runtime directories. Failed or interrupted Fiber payments can leave inflight TLCs that temporarily consume liquidity.

Create a separate recording node set:

```bash
DEMO_SENDER_NODE=record-sender \
DEMO_LSP_NODE=record-lsp \
DEMO_RECIPIENT_NODE=record-recipient \
DEMO_SENDER_RPC_PORT=8927 \
DEMO_SENDER_P2P_PORT=8928 \
DEMO_LSP_RPC_PORT=9027 \
DEMO_LSP_P2P_PORT=9028 \
DEMO_RECIPIENT_RPC_PORT=9127 \
DEMO_RECIPIENT_P2P_PORT=9128 \
scripts/init-demo-nodes.sh
```

Print the CKB testnet addresses for funding:

```bash
scripts/demo-addresses.sh record-sender record-lsp record-recipient
```

Suggested testnet funding for a small `10 CKB` demo:

```text
record-sender:    500 CKB or more
record-lsp:       500 CKB or more
record-recipient: 221 CKB
```

For a larger `100 CKB` demo, fund more generously:

```text
record-sender:    1500 CKB or more
record-lsp:       2500 CKB or more
record-recipient: 221 CKB
```

The recipient funding is not inbound liquidity. It is the reserve needed by current Fiber/CKB channel acceptor mechanics.

## Start The Stack

Use the same node names and ports when starting the demo:

```bash
DEMO_SENDER_NODE=record-sender \
DEMO_LSP_NODE=record-lsp \
DEMO_RECIPIENT_NODE=record-recipient \
DEMO_SENDER_RPC_PORT=8927 \
DEMO_SENDER_P2P_PORT=8928 \
DEMO_LSP_RPC_PORT=9027 \
DEMO_LSP_P2P_PORT=9028 \
DEMO_RECIPIENT_RPC_PORT=9127 \
DEMO_RECIPIENT_P2P_PORT=9128 \
DEMO_LSPD_PORT=3003 \
DEMO_UI_PORT=5174 \
scripts/demo-start.sh
```

Open the UI:

```text
http://127.0.0.1:5174
```

## Screenshot 1: Recipient Has Zero Channels

Before running a payment, capture this terminal output:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"list_channels","params":[{"include_closed":false}],"id":1}' \
  http://127.0.0.1:9127 | jq .
```

Expected clean output:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "channels": []
  }
}
```

This is the first proof point: the recipient did not start with inbound liquidity.

Also take a UI screenshot before payment. The recipient channel table should be empty or show zero before-balance.

## Run The Payment

Use a small amount for the most reliable recording:

```bash
DEMO_SENDER_URL=http://127.0.0.1:8927 \
DEMO_LSPD_URL=http://127.0.0.1:3003 \
DEMO_RECIPIENT_URL=http://127.0.0.1:9127 \
DEMO_RECIPIENT_P2P_PORT=9128 \
scripts/demo-run-payment.sh 1000000000
```

`1000000000` shannons is `10 CKB`.

Wait for:

```text
COMPLETED | recipient paid ...; Fiber invoice settled; LSP fee earned ...
```

If the order stays in `OPENING_CHANNEL`, the usual cause is that the recipient channel funding transaction has not reached ready state yet. Give it more time. If it fails because of insufficient balance, restart with fresh nodes or fund the sender/LSP more generously.

## Screenshot 2: Completed Order

After the script completes, capture the final terminal result and the UI.

The UI should show:

- status `COMPLETED`;
- gross amount, fee, and net amount;
- audit trail from `AWAITING_PAYMENT` to `COMPLETED`;
- recipient channel before/after balance;
- channel outpoint.

## Screenshot 3: Recipient Received Without Pre-Funding Inbound Liquidity

Query the recipient again:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"list_channels","params":[{"include_closed":false}],"id":1}' \
  http://127.0.0.1:9127 | jq .
```

Look for these fields:

```text
state.state_name: ChannelReady
is_acceptor: true
is_one_way: true
local_balance: non-zero recipient balance
channel_outpoint: on-chain funding outpoint
```

Interpretation:

- `is_acceptor: true` means the recipient accepted the channel.
- The LSP side of the same channel has `is_acceptor: false`, which shows the LSP opened/funded it.
- `local_balance` on the recipient side is the Fiber balance received.
- `channel_outpoint` points to the CKB funding transaction for the channel.

## Screenshot 4: Funding Transaction Hash

The recipient channel response includes a `channel_outpoint` like this:

```text
0x5a0964d46f1620af6e5ea590ae304583a9f6eb6a936fa9c57b28434917b054ad00000000
```

Split it as:

```text
funding tx hash: 0x5a0964d46f1620af6e5ea590ae304583a9f6eb6a936fa9c57b28434917b054ad
output index:     00000000
```

Use that transaction hash on a CKB testnet explorer or with CKB RPC to show that an on-chain Fiber channel funding transaction exists.

This proves channel creation. The payment itself is a Fiber off-chain payment, so its proof is the Fiber payment/order state:

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

Check LSP side of the recipient channel:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"list_channels","params":[{"include_closed":false}],"id":1}' \
  http://127.0.0.1:9027 | jq .
```

## What To Say In The Demo

Use this wording:

```text
The recipient starts with zero Fiber channels, so there is no pre-created inbound liquidity.
The sender pays an invoice issued by the LSP.
The LSP then opens a one-way channel to the recipient and pays the recipient net amount over Fiber.
Only after the recipient payment succeeds does the LSP settle the sender invoice.
The channel funding transaction is visible through channel_outpoint, while the Fiber payment result is visible through get_payment and get_order_status.
The recipient still needs a small CKB reserve to accept the channel, so the claim is not zero CKB. The claim is no pre-funded inbound liquidity.
```

## Cleanup

Stop the recording stack:

```bash
DEMO_SENDER_NODE=record-sender \
DEMO_LSP_NODE=record-lsp \
DEMO_RECIPIENT_NODE=record-recipient \
scripts/demo-stop.sh
```

If you want another fully clean recording, create another node set with new names and ports instead of reusing a runtime that has failed or interrupted attempts.
