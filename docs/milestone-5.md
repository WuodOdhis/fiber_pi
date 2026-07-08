# Milestone 5: Watchers

Milestone 5 adds background watchers that poll Fiber state and move orders through the state machine.

The daemon now starts three watcher loops when `lspd` starts:

```text
invoice watcher
channel watcher
```

## Configuration

New environment variables:

```text
POLL_INTERVAL_MS=2000
ORDER_TIMEOUT_SECONDS=7200
```

These are also returned by `get_info`.

## Invoice Watcher

The invoice watcher polls Fiber `get_invoice` for active orders in:

```text
AWAITING_PAYMENT
```

Observed Fiber invoice statuses from `v0.8.1` docs:

```text
Open
Received
Paid
Cancelled
Expired
```

Current mapping:

```text
Open      -> no transition
Received  -> PAYMENT_HELD
Cancelled -> CANCELLED
Expired   -> CANCELLED
Paid      -> FAILED, because it skipped the expected LSP provisioning flow
```

`Received` is the important hold-invoice signal: the payment has arrived but is not settled yet.

## Channel Watcher

The channel watcher polls Fiber `list_channels` for active orders in:

```text
OPENING_CHANNEL
```

Current mapping:

```text
ChannelReady -> CHANNEL_READY
Closed with failure_detail -> FAILED
other channel states -> no transition
```

Important rule: `open_channel` returning `temporary_channel_id` is not enough. The watcher only treats the channel as usable when Fiber reports `state.state_name = "ChannelReady"`.

## Timeout Watcher

The timeout watcher checks active orders against `ORDER_TIMEOUT_SECONDS`.

Current mapping:

```text
AWAITING_PAYMENT -> CANCELLED
PAYMENT_HELD -> FAILED
OPENING_CHANNEL -> FAILED
```

Later, after invoice cancellation is wired in, `PAYMENT_HELD` and `OPENING_CHANNEL` should move through `CANCELLING_INVOICE` instead of directly failing.

## API Impact

`get_order_status` now returns timestamps:

```json
{
  "created_at_ms": 1783530390103,
  "updated_at_ms": 1783530390103
}
```

These timestamps are used by the timeout watcher and are useful for demo logs.

## Verified

Tests:

```text
cargo fmt --all
cargo test
```

Live smoke test:

```text
buy -> AWAITING_PAYMENT
invoice status -> Open
watchers keep order active without invalid transition
```

Observed output:

```text
AWAITING_PAYMENT invoice=Open updated=<timestamp>
```

## Not Included Yet

This milestone does not open channels or settle invoices automatically. It only watches external Fiber state and applies safe transitions.

Next milestone:

```text
Milestone 6: End-To-End Happy Path
```

Before the full happy path can work, the LSP node must be funded or the environment must provide a channel funding path so `open_channel` can reach `ChannelReady`.
