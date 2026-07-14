# GitHub Codespaces Demo

Use this when a VPS is not available. Codespaces can host the full testnet demo stack and expose the browser UI through a forwarded public port.

The live demo runs inside one Codespace:

```text
codespace-sender     Fiber demo sender node
codespace-lsp        Fiber LSP node used by lspd
codespace-recipient  Fiber demo recipient node
lspd                 receive-first LSP daemon
demo-ui              browser dashboard exposed on port 5173
```

Only the UI port needs to be public. The Fiber RPC ports stay local inside the Codespace.

## 1. Create The Codespace

Open the repository on GitHub and create a Codespace on `main`.

The devcontainer installs Rust, Node.js, `jq`, `curl`, OpenSSL headers, Clang, and protobuf tooling. It also runs:

```bash
cargo check -p lspd
```

## 2. Build Fiber

In the Codespaces terminal:

```bash
scripts/prepare-fiber.sh
```

This builds Fiber `v0.8.1` and copies `fnn` and `fnn-cli` into `.fiber-bin/`.

## 3. Create Fresh Demo Nodes

```bash
DEMO_SENDER_NODE=codespace-sender \
DEMO_LSP_NODE=codespace-lsp \
DEMO_RECIPIENT_NODE=codespace-recipient \
scripts/init-demo-nodes.sh
```

This creates ignored runtime directories under `runtime/`.

## 4. Print Funding Addresses

```bash
scripts/demo-addresses.sh codespace-sender codespace-lsp codespace-recipient
```

Fund these testnet addresses:

```text
codespace-sender:    500 CKB or more
codespace-lsp:       500 CKB or more
codespace-recipient: 221 CKB
```

For a larger demo, fund sender and LSP more generously.

The recipient funding is not inbound liquidity. It is the CKB reserve needed by current Fiber/CKB channel acceptor mechanics.

## 5. Start The Live Demo Stack

```bash
scripts/codespaces-demo-start.sh
```

Wait for:

```text
[done] demo stack is ready
```

The script starts the three Fiber nodes, connects peers, checks sender outbound liquidity, verifies recipient channel count, starts `lspd`, and starts the UI on `0.0.0.0:5173`.

## 6. Make Port 5173 Public

In the Codespaces **Ports** tab, find port `5173`.

Set visibility to **Public** if it is not already public.

Open the forwarded URL. This is the live demo URL you can submit to judges.

## 7. Prove Recipient Starts With Zero Channels

Before running the payment:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"list_channels","params":[{"include_closed":false}],"id":1}' \
  http://127.0.0.1:8827 | jq .
```

Expected:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "channels": []
  }
}
```

## 8. Run A Payment

Use `10 CKB` for the most reliable demo:

```bash
scripts/codespaces-demo-pay.sh 1000000000
```

Wait for:

```text
COMPLETED | recipient paid ...; Fiber invoice settled; LSP fee earned ...
```

## 9. Proof Points For Reviewers

After completion, show:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"list_channels","params":[{"include_closed":false}],"id":1}' \
  http://127.0.0.1:8827 | jq .
```

Important fields:

```text
state.state_name: ChannelReady
is_acceptor: true
is_one_way: true
local_balance: non-zero
channel_outpoint: on-chain funding outpoint
```

The `channel_outpoint` contains the CKB funding transaction hash. The first 32 bytes are the tx hash, and the final 4 bytes are the output index.

## 10. Keep The Codespace Alive

Codespaces can stop after inactivity. During judging, keep it running or restart it and run:

```bash
scripts/codespaces-demo-start.sh
```

If the recipient no longer has zero channels because you already ran the demo, create a new node set with new names, fund the new addresses, and start again.

## Submission Note

Use this wording:

```text
The live demo is hosted in GitHub Codespaces. It runs a self-contained Fiber testnet stack: sender node, LSP node, recipient node, lspd daemon, and dashboard. Only the dashboard port is public; Fiber RPC services remain local inside the Codespace. All funds are testnet-only.
```
