# Observability Moat

This project intentionally keeps the hackathon protocol scope narrow. The moat is not extra protocol ambition; it is a clear LSP control plane around Fiber's real RPC behavior.

The daemon now exposes an audit trail for every order.

Each order includes:

```text
status
status_reason
created_at_ms
updated_at_ms
```

Each event includes:

```text
timestamp_ms
status
reason
```

Example status response:

```json
{
  "status": "AWAITING_PAYMENT",
  "status_reason": "hold invoice created",
  "events": [
    {
      "timestamp_ms": 1783530390103,
      "status": "AWAITING_PAYMENT",
      "reason": "hold invoice created"
    }
  ]
}
```

Why this matters:

- Wallets can show meaningful user-facing progress instead of a black-box pending state.
- Judges can see the order lifecycle during the demo.
- Failure states can carry Fiber-specific details such as `Funding transaction aborted`.
- The daemon remains Fiber-native and does not claim unsupported payment interception.

This strengthens the project without changing the implementation plan. The core demo remains:

```text
hold invoice
payment detection
channel opening
ChannelReady polling
invoice settlement
```

The audit trail simply makes that flow inspectable and easier to trust.
