# Milestone 4: Order State Machine

Milestone 4 adds an explicit state machine for every LSP liquidity order.

The goal is to prevent invalid daemon behavior before adding watchers. In particular, the daemon must not settle an invoice before a channel reaches `ChannelReady`, and it must not skip from payment detection directly to completion.

## States

The order lifecycle is now represented by `OrderStatus`:

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

The values serialize as `SCREAMING_SNAKE_CASE`, so API responses match the demo and submission language.

## Happy Path

The intended successful flow is:

```text
CREATED
  -> AWAITING_PAYMENT
  -> PAYMENT_HELD
  -> OPENING_CHANNEL
  -> CHANNEL_READY
  -> SETTLING
  -> COMPLETED
```

## Failure Paths

Supported failure/cancel paths include:

```text
AWAITING_PAYMENT -> CANCELLED
AWAITING_PAYMENT -> FAILED
PAYMENT_HELD -> CANCELLING_INVOICE -> CANCELLED
PAYMENT_HELD -> FAILED
OPENING_CHANNEL -> CANCELLING_INVOICE -> CANCELLED
OPENING_CHANNEL -> FAILED
CHANNEL_READY -> FAILED
SETTLING -> FAILED
CANCELLING_INVOICE -> FAILED
```

Terminal states are:

```text
COMPLETED
CANCELLED
FAILED
```

No transition is allowed out of a terminal state.

## Store Integration

`OrderStore` now has a `transition` method. It validates the current status against the requested next status before changing the order.

Every transition records a `status_reason`, which is returned by the API and will become useful for logs and demo output.

## Current API Behavior

`buy` still creates orders directly in:

```text
AWAITING_PAYMENT
```

It also sets:

```text
status_reason = "hold invoice created"
```

`get_order_status` returns the order status from the state machine instead of a hardcoded string.

## Not Included Yet

This milestone does not add background watchers. It only defines and enforces the lifecycle rules.

Next milestone:

```text
Milestone 5: Implement Watchers
```

The watchers will poll Fiber invoice/channel state and use the state machine to move orders forward.
