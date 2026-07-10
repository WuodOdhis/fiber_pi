# Milestone 6: Happy-Path Orchestration

Milestone 6 adds the daemon-side executor for the intended end-to-end flow.

The full happy path still requires a funded LSP Fiber node and a captured sender payment into the hold invoice. Without spendable testnet cells, Fiber can start a channel open but later reports `FUNDING_ABORTED`.

## Implemented Flow

When an order reaches `PAYMENT_HELD`, the executor watcher now:

1. Moves the order to `OPENING_CHANNEL`.
2. Connects the LSP Fiber node to the recipient peer using `connect_peer`.
3. Calls `open_channel` to the recipient with the net amount after LSP fee.
4. Leaves channel readiness detection to the channel watcher.

When the channel watcher observes `ChannelReady`, the executor watcher now:

1. Moves the order to `SETTLING`.
2. Calls `settle_invoice` with the stored payment hash and preimage.
3. Moves the order to `COMPLETED` after successful settlement.
4. Records the earned LSP fee in the final status reason.

If `connect_peer`, `open_channel`, or `settle_invoice` fails, the order moves to `FAILED` with the Fiber error in the audit trail.

## Status API

`get_order_status` now includes a channel snapshot for the recipient peer:

```json
{
  "channels": [
    {
      "channel_id": "0x...",
      "state": "Closed",
      "state_flags": "FUNDING_ABORTED",
      "local_balance": "0x...",
      "remote_balance": "0x0",
      "failure_detail": "Funding transaction aborted"
    }
  ]
}
```

This keeps the funding blocker visible through the LSP API instead of hiding it in node logs.

## Verification

Unit tests:

```text
cargo fmt --all
cargo test
```

Live smoke against the LSP Fiber node:

```text
AWAITING_PAYMENT invoice=Open events=1 channels=0
```

This confirms the existing `buy` path and enriched status response still work. The executor path waits for `PAYMENT_HELD`, which remains blocked until sender payment routing into the hold invoice is available.

## Remaining Blocker

The remaining work is environmental rather than daemon orchestration:

1. Fund the LSP node's testnet funding lock script, or run a local devnet with Fiber scripts deployed.
2. Capture a sender payment that moves the hold invoice from `Open` to the held state expected by Fiber.
3. Re-run the flow and confirm `ChannelReady -> SETTLING -> COMPLETED`.
