# Milestone 1: Fiber RPC Shapes

This document records observed Fiber `v0.8.1` RPC behavior from the local three-node setup.

The goal is to remove assumptions before writing daemon structs and business logic.

## Environment

Pinned Fiber version:

```text
fnn Fiber v0.8.1 (b560023 2026-04-16)
fnn-cli 0.8.1
```

Local nodes:

```text
sender:    http://127.0.0.1:8327, p2p /ip4/127.0.0.1/tcp/8328
lsp:       http://127.0.0.1:8427, p2p /ip4/127.0.0.1/tcp/8428
recipient: http://127.0.0.1:8527, p2p /ip4/127.0.0.1/tcp/8528
```

Important network constraint:

```text
The generated configs use Fiber testnet settings, so invoice currency must be Fibt.
Fibb is rejected on this chain with: Currency must be Fibt with the chain network.
```

## JSON-RPC Call Shape

Fiber uses standard JSON-RPC 2.0.

For methods with structured parameters, send `params` as an array containing one object:

```json
{
  "jsonrpc": "2.0",
  "method": "new_invoice",
  "params": [
    {
      "amount": "0x174876e800",
      "description": "jsonrpc sha256 hold invoice",
      "currency": "Fibt",
      "payment_hash": "0x...",
      "expiry": "0xe10",
      "hash_algorithm": "sha256"
    }
  ],
  "id": 2
}
```

For methods without parameters, `params: []` works:

```json
{"jsonrpc":"2.0","method":"node_info","params":[],"id":1}
```

## `node_info`

Request:

```json
{"jsonrpc":"2.0","method":"node_info","params":[],"id":1}
```

Observed response shape:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "version": "0.8.1",
    "commit_hash": "b560023 2026-04-16",
    "pubkey": "0358115f048eefb31d99becefc1d07f3c249faac0dcefd24c70055b5fbd411e08d",
    "features": ["GOSSIP_QUERIES_REQUIRED", "BASIC_MPP_REQUIRED", "TRAMPOLINE_ROUTING_REQUIRED"],
    "node_name": null,
    "addresses": [],
    "chain_hash": "0x10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606",
    "open_channel_auto_accept_min_ckb_funding_amount": "0x2540be400",
    "auto_accept_channel_ckb_funding_amount": "0x24e160300",
    "default_funding_lock_script": {
      "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
      "hash_type": "type",
      "args": "0x56af0a41efc21ecd6979f9e67abbb4d554f4180b"
    },
    "tlc_expiry_delta": "0xdbba00",
    "tlc_min_value": "0x0",
    "tlc_fee_proportional_millionths": "0x3e8",
    "channel_count": "0x0",
    "pending_channel_count": "0x0",
    "peers_count": "0x0",
    "udt_cfg_infos": []
  }
}
```

Implementation notes:

- Node identity field is `pubkey`, not `peer_id`.
- Numeric values are returned as hex strings.
- `addresses` can be empty when `announce_listening_addr` is disabled.
- Do not assume `udt_cfg_infos` is empty; the testnet config includes RUSD.

## `connect_peer`

Request shape:

```json
{
  "jsonrpc": "2.0",
  "method": "connect_peer",
  "params": [
    {
      "address": "/ip4/127.0.0.1/tcp/8528",
      "pubkey": "02913dcce1e35f5ffc32ff04a6ddafccf660f5e88293b59c3c80b6dddf7a0a8406",
      "save": true
    }
  ],
  "id": 10
}
```

Observed CLI equivalent:

```bash
.fiber-bin/fnn-cli -u http://127.0.0.1:8427 peer connect_peer \
  --address /ip4/127.0.0.1/tcp/8528 \
  --pubkey 02913dcce1e35f5ffc32ff04a6ddafccf660f5e88293b59c3c80b6dddf7a0a8406 \
  --save true
```

Observed response:

```text
connect_peer returns no JSON result body through fnn-cli on success.
```

Verification is done through `list_peers`.

Observed `list_peers` on the LSP node after connecting sender and recipient:

```json
{
  "peers": [
    {
      "address": "/ip4/127.0.0.1/tcp/8528/p2p/QmNsCL55X3BHh5TSU2MZXLrkv4n3AbffSkyEqyRsyQsbSV",
      "pubkey": "02913dcce1e35f5ffc32ff04a6ddafccf660f5e88293b59c3c80b6dddf7a0a8406"
    },
    {
      "address": "/ip4/127.0.0.1/tcp/<ephemeral>/p2p/QmcoJE2SdJLDKzzGNhVRYJGX6W5HsHzRT4Vs9rMmhGUsRJ",
      "pubkey": "03670d0056803d9cf7a0ed9ba8288d1214fe7e2df0bf06629c6cd28e1530aacbfc"
    }
  ]
}
```

Implementation notes:

- LSP-to-recipient should call `connect_peer` before `open_channel`.
- The returned peer address may include `/p2p/<peer-id>` even if the request address did not.
- Incoming peer connections may show ephemeral local ports.

## `new_invoice`: Hold Invoice

Hold invoice rule from Fiber docs:

```text
Set payment_hash and omit payment_preimage.
```

Observed gotcha:

```text
If hash_algorithm is omitted, Fiber defaults to ckb_hash.
If the daemon computes a SHA-256 payment hash, it must set hash_algorithm = sha256.
```

Request:

```json
{
  "jsonrpc": "2.0",
  "method": "new_invoice",
  "params": [
    {
      "amount": "0x174876e800",
      "description": "jsonrpc sha256 hold invoice",
      "currency": "Fibt",
      "payment_hash": "0x4e0a438e3f44580d0ded5fb304420a8f085f2b9523f2c25dee10149c45cd4c7f",
      "expiry": "0xe10",
      "hash_algorithm": "sha256"
    }
  ],
  "id": 2
}
```

Observed response shape:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "invoice_address": "fibt1000000000001p...",
    "invoice": {
      "currency": "Fibt",
      "amount": "0x174876e800",
      "signature": "...",
      "data": {
        "timestamp": "0x19f428932f2",
        "payment_hash": "0x4e0a438e3f44580d0ded5fb304420a8f085f2b9523f2c25dee10149c45cd4c7f",
        "attrs": [
          {"description": "jsonrpc sha256 hold invoice"},
          {"expiry_time": "0xe10"},
          {"final_htlc_minimum_expiry_delta": "0x927c00"},
          {"hash_algorithm": "sha256"},
          {"payee_public_key": "0358115f048eefb31d99becefc1d07f3c249faac0dcefd24c70055b5fbd411e08d"}
        ]
      }
    }
  }
}
```

Implementation notes:

- Amounts are hex strings in JSON-RPC examples.
- `Fibt` is required for the current testnet-backed config.
- For demo simplicity, generate a random 32-byte preimage, compute SHA-256, pass the hash as `payment_hash`, and set `hash_algorithm` to `sha256`.
- Store the preimage locally in the LSP order record; it is needed later for `settle_invoice`.

## `get_invoice`

Request:

```json
{
  "jsonrpc": "2.0",
  "method": "get_invoice",
  "params": [
    {
      "payment_hash": "0x4e0a438e3f44580d0ded5fb304420a8f085f2b9523f2c25dee10149c45cd4c7f"
    }
  ],
  "id": 3
}
```

Observed response status for a newly created hold invoice:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "invoice_address": "fibt1000000000001p...",
    "invoice": {
      "currency": "Fibt",
      "amount": "0x174876e800",
      "data": {
        "payment_hash": "0x4e0a438e3f44580d0ded5fb304420a8f085f2b9523f2c25dee10149c45cd4c7f",
        "attrs": [
          {"hash_algorithm": "sha256"},
          {"payee_public_key": "0358115f048eefb31d99becefc1d07f3c249faac0dcefd24c70055b5fbd411e08d"}
        ]
      }
    },
    "status": "Open"
  }
}
```

Implementation notes:

- The invoice watcher should poll `get_invoice` by `payment_hash`.
- Initial hold invoice state is `Open`.
- The paid-but-held state is not yet captured because sender payment requires a usable route/channel.

## `settle_invoice`

Request shape:

```json
{
  "jsonrpc": "2.0",
  "method": "settle_invoice",
  "params": [
    {
      "payment_hash": "0x...",
      "payment_preimage": "0x..."
    }
  ],
  "id": 4
}
```

Observed behavior with correct SHA-256 preimage/hash pair while invoice is still `Open`:

```text
Error: RPC error (code -32000): Invoice is still open
```

Observed behavior when the hash was computed with SHA-256 but `hash_algorithm` was omitted during invoice creation:

```text
Error: RPC error (code -32000): Hash mismatch
```

Implementation notes:

- This confirms `settle_invoice` checks both status and preimage/hash validity.
- Do not call `settle_invoice` while invoice status is `Open`.
- The daemon should only settle after `get_invoice` shows the paid/held state, which still needs capture after channel/payment routing works.

## `list_channels`

Request shape:

```json
{
  "jsonrpc": "2.0",
  "method": "list_channels",
  "params": [
    {
      "only_pending": true
    }
  ],
  "id": 5
}
```

Observed empty response before channel attempts:

```json
{
  "channels": []
}
```

Observed failed funding record after `open_channel` without funded testnet cells:

```json
{
  "channels": [
    {
      "channel_id": "0x0ae283de086bf7358b7339cf151b471958fed107d4c20692d0a1a47ec98d2aab",
      "channel_outpoint": null,
      "enabled": false,
      "failure_detail": "Funding transaction aborted",
      "funding_udt_type_script": null,
      "is_acceptor": false,
      "is_one_way": false,
      "is_public": false,
      "local_balance": "0x174876e800",
      "remote_balance": "0x0",
      "pending_tlcs": [],
      "pubkey": "02913dcce1e35f5ffc32ff04a6ddafccf660f5e88293b59c3c80b6dddf7a0a8406",
      "state": {
        "state_flags": "FUNDING_ABORTED",
        "state_name": "Closed"
      }
    }
  ]
}
```

Implementation notes:

- Channel state is nested at `state.state_name` and `state.state_flags`.
- Failed funding attempts remain visible through `list_channels --only-pending true`.
- `failure_detail` must be surfaced in LSP order failure logs.
- `ChannelReady` detection should check `state.state_name == "ChannelReady"` once a funded channel can be opened.

## `open_channel`

Request shape:

```json
{
  "jsonrpc": "2.0",
  "method": "open_channel",
  "params": [
    {
      "pubkey": "02913dcce1e35f5ffc32ff04a6ddafccf660f5e88293b59c3c80b6dddf7a0a8406",
      "funding_amount": "0x174876e800"
    }
  ],
  "id": 6
}
```

Observed CLI response when LSP is connected to recipient:

```json
{
  "temporary_channel_id": "0x53be3c5262794e3973a079fa0828d18b7007f77a487da4608a2f5fbdcd844604"
}
```

Observed follow-up state without funded testnet cells:

```text
state.state_name = Closed
state.state_flags = FUNDING_ABORTED
failure_detail = Funding transaction aborted
```

Implementation notes:

- `open_channel` can return success before funding has actually completed.
- The daemon must not treat `temporary_channel_id` as channel readiness.
- After `open_channel`, the daemon must poll `list_channels` until either `ChannelReady` or a failure state is observed.
- Funding is now the main blocker for a complete channel-ready demo.

## Confirmed Design Implications

1. The daemon should use `pubkey`, not `peer_id`, for Fiber `v0.8.1` node identity.
2. The LSP API can still call the client-facing field `recipient_pubkey`.
3. The daemon should store amounts as strings or parse Fiber hex quantities carefully.
4. For the current testnet config, use `Fibt` as the CKB testnet invoice currency.
5. Use `hash_algorithm: "sha256"` if generating SHA-256 hold invoice hashes in the daemon.
6. Store payment preimages locally per order; they are required for settlement.
7. `settle_invoice` is only valid after invoice status is no longer `Open`.
8. `open_channel` success only means an opening attempt started; readiness requires `list_channels` polling.
9. Funding failure is visible through `state.state_flags = "FUNDING_ABORTED"` and `failure_detail`.

## Remaining Blockers For Full Happy Path

1. Fund the LSP node's testnet funding lock script or establish a local devnet with deployed Fiber scripts.
2. Capture successful channel state progression from opening to `ChannelReady`.
3. Capture sender `send_payment` to a hold invoice.
4. Capture the invoice state after payment arrives and is held.
5. Capture successful `settle_invoice` after the held payment exists.

These blockers must be resolved before Milestone 6, but they do not block Milestone 2's RPC client skeleton.
