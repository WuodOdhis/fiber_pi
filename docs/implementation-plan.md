# Fiber LSP Hackathon Implementation Plan

This file is the source of truth for the build. Before adding a feature,
changing scope, or rewriting architecture, return here and check whether the
change supports the demo target.

No assumptions are allowed. If a Fiber RPC method, invoice behavior, channel
state, funding path, or devnet setup detail is uncertain, verify it against a
running `fnn` node before building on top of it.

## Demo Target

Build a narrow, working Fiber-native LSP daemon that helps a new recipient
receive a first payment without pre-funding a channel.

Successful demo flow:

```text
sender pays invoice
LSP detects held/pending payment
LSP opens channel to recipient
channel reaches ChannelReady
LSP settles invoice through Fiber
recipient receives net amount after fee
LSP logs fee earned
```

The project is LSPS2-inspired, but it is not a direct Lightning LSPS2 port.
Lightning uses HTLC-oriented interception flows. Fiber uses PTLC/TLC mechanics
internally, and the daemon interacts with Fiber only through RPC.

## Hard Scope

The hackathon MVP includes only these pieces:

1. One LSP daemon.
2. One LSP Fiber node.
3. One sender Fiber node.
4. One recipient Fiber node.
5. Local Fiber node setup.
6. LSP service RPC for `get_info`, `buy`, and order status.
7. Fiber RPC orchestration for invoice creation, payment detection, channel opening, channel readiness polling, and settlement.
8. Fee calculation and visible logs.
9. A repeatable demo script or CLI.
10. Submission docs, screenshots, and video.

## Explicit Non-Goals

Do not build these during the hackathon MVP:

1. Decentralized liquidity marketplace.
2. On-chain matching contracts.
3. Trustless liquidity escrow contracts.
4. Multiple competing LSPs.
5. Production-grade wallet UI.
6. Native payment interception inside Fiber.
7. Custom PTLC/TLC implementation.
8. Fiber node fork or patch.
9. Automatic rebalancing.
10. Multi-asset support beyond the simplest working asset.

These can appear only in the future work section of the submission.

## Architecture Boundary

The LSP daemon is an application-layer service beside a Fiber node.

Fiber owns:

- channel state;
- invoice lifecycle;
- payment routing;
- PTLC/TLC settlement internals;
- on-chain funding and settlement interaction;
- peer networking.

The LSP daemon owns:

- LSP fee policy;
- liquidity request records;
- order state machine;
- Fiber RPC calls;
- polling and timeout policy;
- logs and demo observability;
- client-facing `get_info`, `buy`, and order status endpoints.

## Required Fiber RPC Methods

The implementation depends on these Fiber RPC methods being available and
working in the chosen Fiber version:

- `node_info`
- `connect_peer`
- `new_invoice`
- `get_invoice`
- `settle_invoice`
- `open_channel`
- `list_channels`
- `get_payment`, if needed for sender/payment status checks

Do not assume response schemas. Capture real response examples from the running
node and adapt typed structs to those examples.

## Verified Milestone 0 Facts

Milestone 0 verifies local Fiber node setup before daemon implementation.

Current verified facts:

- Official Fiber repo: `https://github.com/nervosnetwork/fiber`
- Pinned Fiber version: `v0.8.1`
- Local source build of `fnn` and `fnn-cli` succeeds.
- Published Docker images were not pullable from this environment, so local source build is the fallback.
- Three local `fnn` processes start successfully with generated demo keys.
- All three local RPC endpoints respond to `fnn-cli info`.
- Direct local peer connections are verified:
  - sender connects to LSP;
  - LSP connects to recipient.

Demo node ports:

```text
sender:    p2p 8328, rpc 8327
lsp:       p2p 8428, rpc 8427
recipient: p2p 8528, rpc 8527
```

Important constraint: the official Fiber `config/testnet/config.yml` points to
public CKB testnet and includes deployed Fiber script dependencies. A pure local
CKB devnet is not automatically equivalent unless Fiber scripts are deployed to
that devnet. For now, local Fiber node topology is verified with Fiber's testnet
script configuration.

## Unknowns To Verify Before Daemon Logic

These must be tested locally against `fnn` before we rely on them:

1. Exact `new_invoice` parameters for creating a held/pending invoice.
2. Whether `settle_invoice` requires a preimage, payment key, secret, or another Fiber-specific settlement parameter.
3. Exact invoice states returned by `get_invoice`.
4. Whether an invoice can remain pending long enough for channel confirmation.
5. Exact `open_channel` request parameters and funding requirements.
6. Exact channel states returned by `list_channels` and the field that identifies `ChannelReady`.
7. How local nodes are funded in the chosen environment.
8. Whether external wallet signing is required for channel funding in the local setup.
9. Whether sender payment can route to the LSP-held invoice before the recipient channel exists.

If any of these fail, adjust the demo flow before writing more daemon logic.

## Repository Structure Target

Use this structure as the project grows:

```text
fiber_pi/
  docs/
    implementation-plan.md
    milestone-0.md
    demo-runbook.md
    rpc-examples.md
  scripts/
    check-env.sh
    prepare-fiber.sh
    init-demo-nodes.sh
    start-node.sh
    probe-demo-nodes.sh
    connect-demo-peers.sh
  crates/
    lspd/
      src/
        main.rs
        config.rs
        error.rs
        fiber_rpc.rs
        lsp_api.rs
        model.rs
        order_store.rs
        state_machine.rs
        fee.rs
        watchers.rs
    demo-cli/
      src/
        main.rs
```

Keep this minimal. Do not add a database unless in-memory state blocks the demo.
For the hackathon, a JSON file store is enough if persistence is needed.

## Implementation Milestones

### Milestone 0: Verify Fiber Environment

Status: complete.

Acceptance:

```text
fnn binary exists locally
fnn-cli binary exists locally
sender, lsp, and recipient node directories exist
each node has its own config.yml
each node has its own ckb/key file
each node has a unique Fiber P2P port
each node has a unique RPC port
all three nodes start and respond to fnn-cli info
peer connection commands are documented and verified
```

### Milestone 1: Capture Real Fiber RPC Shapes

Goal: remove schema assumptions.

Tasks:

1. Call `node_info` and save request/response.
2. Call `connect_peer` and save request/response.
3. Call `new_invoice` and save request/response.
4. Call `get_invoice` and save invoice states.
5. Call `open_channel` and save request/response.
6. Call `list_channels` and save channel state examples.
7. Call `settle_invoice` in the simplest valid case and save requirements.
8. Store examples in `docs/rpc-examples.md`.

Acceptance:

```text
all daemon structs can be based on observed RPC payloads
hold/pending invoice behavior is understood
settlement requirements are known
ChannelReady detection field is known
```

### Milestone 2: Build Fiber RPC Client

Goal: create a typed Rust wrapper around required Fiber RPC methods.

Tasks:

1. Implement JSON-RPC request helper.
2. Implement `node_info` wrapper.
3. Implement `connect_peer` wrapper.
4. Implement `new_invoice` wrapper.
5. Implement `get_invoice` wrapper.
6. Implement `open_channel` wrapper.
7. Implement `list_channels` wrapper.
8. Implement `settle_invoice` wrapper.
9. Add structured logging for every RPC call.

Acceptance:

```text
lspd can call each required Fiber RPC method against a running node
errors are logged with method name and request id
no mocked Fiber responses in daemon path
```

### Milestone 3: Build LSP API

Goal: expose the small API wallet developers would use.

Methods:

```text
get_info
buy
get_order_status
```

`get_info` returns LSP node id, fee rate, minimum amount, maximum amount,
supported asset, and service version.

`buy` accepts recipient node details and amount, then returns order id, invoice,
gross amount, fee amount, net amount, expiry, and initial status.

Acceptance:

```text
demo CLI can call get_info
demo CLI can call buy
daemon creates an order record
daemon returns an invoice and fee breakdown
```

### Milestone 4: Implement Order State Machine

Goal: make every step explicit and observable.

States:

```text
CREATED
AWAITING_PAYMENT
PAYMENT_HELD
OPENING_CHANNEL
CHANNEL_READY
SETTLING
COMPLETED
CANCELLING_INVOICE
CANCELLED
FAILED
```

Rules:

1. No state transition without a logged reason.
2. No settlement before `ChannelReady`.
3. No channel opening before payment is detected as held/pending.
4. If channel opening fails, cancel or expire the invoice.
5. If timeout occurs, move to a visible failure/cancel state.

### Milestone 5: Implement Watchers

Goal: coordinate asynchronous Fiber state changes.

Watchers:

1. Invoice watcher polls `get_invoice` for active orders.
2. Channel watcher polls `list_channels` for orders in `OPENING_CHANNEL`.
3. Timeout watcher cancels or fails stale orders.

Acceptance:

```text
payment detection moves order to PAYMENT_HELD
channel readiness moves order to CHANNEL_READY
timeouts are visible in logs
poll interval is configurable
```

### Milestone 6: End-To-End Happy Path

Goal: complete the actual demo flow.

Tasks:

1. Recipient requests liquidity through demo CLI.
2. LSP creates invoice and order.
3. Sender pays invoice.
4. Daemon detects payment state.
5. Daemon opens channel to recipient.
6. Daemon waits for `ChannelReady`.
7. Daemon settles invoice.
8. Daemon logs final fee and status.

Acceptance:

```text
new recipient starts without usable inbound channel
sender payment enters held/pending state
LSP opens recipient channel
invoice settlement completes
order status is COMPLETED
fee math is visible
```

### Milestone 7: Demo Polish

Goal: make the project judge-friendly.

Tasks:

1. Add quickstart docs after the implementation stabilizes.
2. Add architecture diagram.
3. Add demo runbook.
4. Add screenshots.
5. Record video.
6. Add MIT license.
7. Add `.env.example`.
8. Add hosted walkthrough page if time permits.

## Minimal API Draft

### `get_info`

Request:

```json
{"jsonrpc":"2.0","method":"get_info","params":{},"id":1}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "service": "fiber-lsp-daemon",
    "version": "0.1.0",
    "fee_rate_bps": 100,
    "min_amount": "100000000000",
    "max_amount": "10000000000000",
    "asset": "CKB",
    "lsp_pubkey": "TBD_FROM_FIBER_NODE"
  },
  "id": 1
}
```

### `buy`

Request:

```json
{
  "jsonrpc": "2.0",
  "method": "buy",
  "params": {
    "recipient_pubkey": "TBD",
    "recipient_address": "TBD",
    "amount": "100000000000"
  },
  "id": 2
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "order_id": "TBD",
    "invoice": "TBD_FROM_FIBER",
    "gross_amount": "100000000000",
    "fee_amount": "1000000000",
    "net_amount": "99000000000",
    "status": "AWAITING_PAYMENT",
    "expires_at": "TBD"
  },
  "id": 2
}
```

### `get_order_status`

Request:

```json
{
  "jsonrpc": "2.0",
  "method": "get_order_status",
  "params": {"order_id": "TBD"},
  "id": 3
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "order_id": "TBD",
    "status": "OPENING_CHANNEL",
    "invoice_state": "TBD_FROM_FIBER",
    "channel_state": "TBD_FROM_FIBER",
    "channel_id": "TBD_IF_AVAILABLE"
  },
  "id": 3
}
```

## Fee Model

Use basis points for demo clarity.

```text
fee_rate_bps = configured value, e.g. 100 for 1 percent
fee_amount = gross_amount * fee_rate_bps / 10000
net_amount = gross_amount - fee_amount
channel_capacity = gross_amount unless Fiber requires a different minimum
```

If Fiber RPC expects shannons or another unit, use that exact unit internally and
format CKB only in logs.

## Demo Script Target Output

The final demo should print something close to this:

```text
[LSP] get_info -> fee_rate_bps=100 asset=CKB
[CLI] buy -> recipient=TBD amount=1000 CKB
[LSP] order created -> AWAITING_PAYMENT
[LSP] invoice created -> <invoice>
[SENDER] paying invoice
[LSP] invoice state -> PAYMENT_HELD
[LSP] opening channel -> recipient=TBD capacity=1000 CKB
[LSP] channel state -> NegotiatingFunding
[LSP] channel state -> ChannelReady
[LSP] settling invoice through Fiber
[LSP] order completed -> recipient_net=990 CKB fee=10 CKB
```

Do not print `HTLC settled` or `preimage revealed` unless Fiber's observed RPC
actually exposes that wording. Use `settle invoice through Fiber` because Fiber
owns the PTLC/TLC internals.

## Hosted Setup Plan

Do not host real Fiber nodes publicly unless it becomes trivial and stable.

Preferred hosted setup:

1. Static project dashboard.
2. Architecture overview.
3. Interactive state-machine walkthrough.
4. API examples.
5. Screenshots.
6. Demo video embed.
7. GitHub link.
8. Clear note that real Fiber execution is reproduced locally from the repo.

## Daily Work Order

Follow this order unless a verified blocker forces a change:

1. Verify Fiber local environment.
2. Capture real RPC shapes.
3. Build Rust Fiber RPC client.
4. Build LSP API skeleton.
5. Implement fee model and order store.
6. Implement state machine.
7. Implement invoice watcher.
8. Implement channel watcher.
9. Wire end-to-end happy path.
10. Add demo CLI.
11. Add logs and runbook.
12. Record demo.
13. Complete submission.

## Risk Register

| Risk | Impact | Response |
|---|---|---|
| Hold invoice behavior differs from assumption | High | Verify before daemon build |
| `settle_invoice` requires unavailable parameter | High | Adjust flow based on real RPC docs/behavior |
| Channel opening requires testnet funding | High | Verify address/funding path before daemon build |
| Channel opening takes too long | Medium | Use local/testnet settings and show pending state clearly |
| API schemas change | Medium | Pin Fiber version in docs and code |
| Hosted live backend is fragile | Medium | Host walkthrough page, prove real flow in video |

## One-Sentence Scope Reminder

This project is a minimal Fiber-native LSP daemon for first-payment inbound
liquidity using existing Fiber RPC, not a decentralized marketplace and not a
custom channel protocol.
