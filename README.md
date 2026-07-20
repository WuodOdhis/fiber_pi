# Fiber LSP

Fiber LSP is a prototype liquidity service for receive-first payments on the Fiber Network.

The problem is simple: a new wallet, merchant, or service may want to receive a Fiber payment before it has inbound liquidity. This daemon sits beside a funded Fiber node and acts as a lightweight LSP. It sells a sender-facing Fiber invoice, provisions or reuses recipient liquidity, pays the recipient over Fiber, and then settles the sender invoice.

This is an MVP, not a production LSP. It is useful because it demonstrates the core receive-first flow with real Fiber RPC calls and real testnet channel state, while keeping the design small enough for wallets and services to inspect or adapt.

The project should be read as a skeleton for a future Fiber LSP service. It proves the orchestration boundary today, then leaves clear upgrade paths for persistence, policy, wallet integration, and deeper protocol-native payment interception as Fiber exposes more application hooks.

## What Works

The current daemon can:

- create a Fiber hold invoice for a sender;
- track the invoice until the sender payment is received;
- connect the LSP Fiber node to the recipient node;
- open a one-way recipient channel when recipient capacity is missing;
- reuse an existing recipient channel when it already has enough local LSP balance;
- pay the recipient the net amount with Fiber keysend;
- settle the sender invoice only after the recipient payment succeeds;
- expose order status, audit events, and recipient channel snapshots over JSON-RPC.

The demo UI shows the same flow in a browser: order status, fee/net amounts, recipient balance before and after, audit trail, and recipient channel outpoint.

## Payment Flow

1. A client calls `buy` with a recipient Fiber pubkey and amount.
2. The LSP daemon asks its Fiber node to create a sender-facing hold invoice.
3. The sender pays that invoice over Fiber.
4. The daemon observes the invoice move to `Received`.
5. The daemon checks whether the LSP already has enough channel balance toward the recipient.
6. If not, the LSP opens a private one-way Fiber channel to the recipient.
7. Once the channel is ready, the LSP sends the recipient the net amount.
8. After the recipient payment succeeds, the daemon settles the original sender invoice.

The important property is ordering: the sender invoice is not settled until the recipient-side Fiber payment has completed.

## What This Proves

For a successful order, there are three useful proof points.

First, the sender-side payment succeeds:

```text
sender get_payment(payment_hash) => Success
```

Second, the LSP order completes:

```text
get_order_status(order_id) => COMPLETED
invoice_status => Paid
```

Third, the recipient receives Fiber balance in a channel it did not pre-create:

```text
recipient list_channels before payment => []
recipient list_channels after payment => ChannelReady
recipient channel is_acceptor => true
recipient channel is_one_way => true
recipient local_balance increases by the net payment amount
```

The on-chain funding transaction for the recipient channel can be read from `channel_outpoint`. The first 32 bytes are the CKB transaction hash; the final 4 bytes are the output index.

Example:

```text
channel_outpoint: 0x5a0964d46f1620af6e5ea590ae304583a9f6eb6a936fa9c57b28434917b054ad00000000
funding tx hash: 0x5a0964d46f1620af6e5ea590ae304583a9f6eb6a936fa9c57b28434917b054ad
output index: 00000000
```

The Fiber payment itself is off-chain. The funding transaction proves the channel was created on CKB; Fiber RPC state proves the payment and settlement outcome.

## Reviewer Evidence

The screenshots below show the demo state that matters for review:

- [Demo Runbook Checklist](docs/screenshots/demo-runbook-checklist.png)
- [Recipient Zero-Channel Proof](docs/screenshots/recipient-zero-channel-proof.png)
- [LSP Daemon API Listening](docs/screenshots/lspd-api-listening.png)
- [Demo UI Before Payment](docs/screenshots/demo-ui-before-payment.png)
- [First Receive Completed](docs/screenshots/demo-ui-first-receive-completed.png)
- [Audit Trail and Channel Outpoint](docs/screenshots/demo-ui-audit-and-outpoint.png)
- [Codespaces Demo Startup](docs/screenshots/codespaces-demo-startup.png)

The key screenshot is [First Receive Completed](docs/screenshots/demo-ui-first-receive-completed.png): it shows a fresh recipient moving from `0 CKB` local balance and no channel to `9.9 CKB` local balance after the LSP provisions the channel and pays the recipient. [Audit Trail and Channel Outpoint](docs/screenshots/demo-ui-audit-and-outpoint.png) shows the order reaching `COMPLETED`, the one-way `ChannelReady` recipient channel, and the CKB funding outpoint.

## What This Does Not Claim

This project does not claim that the recipient needs no CKB at all.

In the current Fiber testnet behavior, the recipient still needs a small CKB reserve to accept channels and maintain the required cells. The useful claim is narrower and more accurate:

```text
The recipient does not pre-fund inbound liquidity.
The LSP funds/provisions the recipient-side channel when payment demand arrives.
```

This project also does not implement a Fiber protocol fork, native payment interception, custom PTLC/TLC logic, or a marketplace. It uses Fiber node JSON-RPC as it exists today.

## Recipient Reserve vs Inbound Liquidity

The distinction between recipient reserve and inbound liquidity is central to the project.

Inbound liquidity is payment capacity in a Fiber channel that lets the recipient receive. In the demo, the recipient starts without that channel capacity. The LSP creates it when there is payment demand.

The CKB reserve is different. Current Fiber/CKB channel acceptance still uses CKB cells and capacity rules. A recipient needs enough CKB capacity to accept and maintain the channel-side cells required by the protocol. That reserve is not the payment liquidity being sold to the sender; it is operating capacity for the recipient node.

In short:

```text
Not claimed: recipient can receive with zero CKB.
Claimed: recipient does not pre-fund inbound Fiber liquidity.
```

This matters for wallets and merchants because the UX problem is inbound liquidity management, not the existence of CKB capacity as a chain resource.

## Repository Layout

```text
crates/lspd/       LSP daemon and JSON-RPC API
demo-ui/           Local browser dashboard for the demo flow
scripts/           Fiber build, demo node setup, start/stop, and payment scripts
RAILWAY.md         Railway bind-port and deployment notes
railway.json       Railway build/start configuration
```

Runtime data, logs, Fiber binaries, downloaded Fiber source, and local CKB tooling are ignored by Git.

## Requirements

- Rust toolchain with `cargo` and `rustc`
- `git`
- `jq`
- `curl`
- Node.js 20 or newer for the demo UI
- CKB testnet funds for the demo nodes

The scripts build Fiber `v0.8.1` locally from the Nervos repository.

## Build

Check local requirements:

```bash
scripts/check-env.sh
```

Build the LSP daemon:

```bash
cargo check -p lspd
```

Build local Fiber binaries:

```bash
scripts/prepare-fiber.sh
```

## Demo Setup

Initialize three local Fiber runtime directories:

```bash
scripts/init-demo-nodes.sh
```

This creates:

```text
runtime/sender     RPC 8627, P2P 8628
runtime/lsp        RPC 8727, P2P 8728
runtime/recipient  RPC 8827, P2P 8828
```

Before running the full demo, fund the three generated CKB keys on testnet. The exact funding amount depends on the testnet state and payment size, but the LSP must have enough CKB to open recipient channels, the sender must have enough CKB to open a channel to the LSP, and the recipient must have enough reserve to accept the channel.

Start the demo stack:

```bash
scripts/demo-start.sh
```

This starts the three Fiber nodes, connects peers, ensures sender-to-LSP liquidity, starts `lspd`, and starts the local UI.

Open the UI:

```text
http://127.0.0.1:5173
```

Run a scripted payment:

```bash
scripts/demo-run-payment.sh 1000000000
```

Amounts are in shannons. `1000000000` is `10 CKB`.

Stop the demo stack:

```bash
scripts/demo-stop.sh
```

## JSON-RPC API

The daemon exposes JSON-RPC over HTTP.

### `get_info`

Returns daemon configuration and LSP Fiber node information.

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"get_info","params":{},"id":1}' \
  http://127.0.0.1:3002
```

### `buy`

Creates an order and a sender-facing Fiber invoice.

```json
{
  "recipient_pubkey": "<recipient fiber pubkey>",
  "recipient_address": "/ip4/127.0.0.1/tcp/8828",
  "amount": "1000000000"
}
```

### `get_order_status`

Returns the current order state, fee/net amounts, invoice status, audit events, and recipient channel snapshots.

```json
{
  "order_id": "<order id>"
}
```

## Operational Notes

Fiber channel and payment state persists in each node runtime directory. Failed or interrupted payment attempts can leave inflight TLCs that consume channel liquidity until Fiber clears them. For a clean first-receive demo, start with fresh runtime directories or wait for stale inflight payments to expire.

The demo uses a local password default for generated demo keys. Do not reuse the demo runtime directories or keys for production funds.

The first run on a fresh recipient proves the receive-first path. Running the demo again against the same recipient no longer proves a zero-channel starting point, because the recipient may already have a channel from the previous run. A repeated run still demonstrates the channel-reuse path. To demonstrate the first-receive path again, create a fresh recipient runtime and fund its CKB reserve.

## Current Limitations

- Orders are stored in memory. Restarting the daemon loses order records.
- Channel funding policy is intentionally simple and CKB-focused.
- There is no authentication on the demo JSON-RPC API.
- Retry and recovery behavior is minimal.
- Liquidity accounting is good enough for the demo path, not yet a production treasury system.
- The recipient still needs a small CKB reserve to accept Fiber channels.

## Integration Roadmap

The current daemon is intentionally small, but it is structured around an integration shape that can grow into a reusable Fiber infrastructure component.

Near-term improvements:

- persistent order storage, so daemon restarts do not lose order state;
- authenticated API access for wallets, merchant backends, and hosted services;
- configurable LSP policy for fees, min/max order size, channel size, and CKB reserve assumptions;
- better retry and recovery for interrupted channel openings or recipient payments;
- richer liquidity accounting across multiple sender and recipient channels;
- SDK-style examples for wallet and merchant integration.

Future Fiber-native improvements:

- native payment interception or event hooks, if/when Fiber exposes them;
- replacing polling-heavy watcher logic with subscription/event-driven order execution;
- tighter integration with route hints or LSP advertisements;
- more direct support for receive-first wallet onboarding flows.

If Fiber later exposes protocol-supported interception or hold-payment callbacks, this daemon can move from the current RPC-orchestrated design toward a cleaner native LSP model. The business logic would remain similar: detect payment demand, provision liquidity, pay the recipient, then settle the sender. The main change would be that Fiber itself would provide a more precise event boundary for the LSP to act on.

## Why This Matters

Receive-first liquidity is a real adoption issue for wallets and merchants. A user should not have to understand inbound liquidity before receiving a first payment. This prototype shows one practical integration path for Fiber: a wallet, merchant backend, or node operator can delegate just-in-time recipient liquidity to an LSP without changing the Fiber protocol.

The next steps toward production are persistence, policy configuration, authenticated APIs, better retry handling, richer liquidity accounting, and wallet-facing SDK bindings.
