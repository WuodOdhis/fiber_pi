# GitHub Codespaces Deployment

GitHub Codespaces can run the full Fiber testnet demo stack without a separate VPS. The deployment is intended for a short-lived live demo environment, not permanent hosting.

Codespaces can sleep or restart after inactivity. The demo can be restored by rerunning the start command, but runtime channel state is persistent inside the Codespace while the runtime directories remain. This means a recipient that has already received once is no longer a clean zero-channel recipient.

The Codespace runs:

```text
codespace-sender     Fiber demo sender node
codespace-lsp        Fiber LSP node used by lspd
codespace-recipient  Fiber demo recipient node
lspd                 receive-first LSP daemon
demo-ui              browser dashboard exposed on port 5173
```

Only the UI port needs public visibility. Fiber RPC ports remain local inside the Codespace.

## Create A Codespace

Create a Codespace on the repository `main` branch. The devcontainer installs Rust, Node.js, `jq`, `curl`, OpenSSL headers, Clang, and protobuf tooling.

The devcontainer also runs:

```bash
cargo check -p lspd
```

## Build Fiber

```bash
scripts/prepare-fiber.sh
```

This builds Fiber `v0.8.1` and copies `fnn` and `fnn-cli` into `.fiber-bin/`.

## Create Demo Nodes

```bash
DEMO_SENDER_NODE=codespace-sender \
DEMO_LSP_NODE=codespace-lsp \
DEMO_RECIPIENT_NODE=codespace-recipient \
scripts/init-demo-nodes.sh
```

This creates ignored runtime directories under `runtime/`.

## Install ckb-cli If Needed

`scripts/demo-addresses.sh` uses `ckb-cli` to derive testnet addresses. If `ckb-cli` is missing, install the Linux release:

```bash
mkdir -p .ckb-cli
curl -L \
  https://github.com/nervosnetwork/ckb-cli/releases/download/v2.0.0/ckb-cli_v2.0.0_x86_64-unknown-linux-gnu.tar.gz \
  -o /tmp/ckb-cli.tar.gz
tar -xzf /tmp/ckb-cli.tar.gz -C .ckb-cli
chmod +x .ckb-cli/ckb-cli_v2.0.0_x86_64-unknown-linux-gnu/ckb-cli
```

## Print Funding Addresses

```bash
scripts/demo-addresses.sh codespace-sender codespace-lsp codespace-recipient
```

Suggested testnet funding for a `10 CKB` payment:

```text
codespace-sender:    500 CKB or more
codespace-lsp:       500 CKB or more
codespace-recipient: 221 CKB
```

The recipient funding is not inbound liquidity. It is the CKB reserve needed by current Fiber/CKB channel acceptor mechanics.

## Start The Stack

```bash
scripts/codespaces-demo-start.sh
```

The command starts the three Fiber nodes, connects peers, checks sender outbound liquidity, verifies recipient channel count, starts `lspd`, and starts the UI on `0.0.0.0:5173`.

Successful startup prints:

```text
[done] demo stack is ready
```

## Expose The UI

In the Codespaces **Ports** tab, set port `5173` to public visibility and open the forwarded URL.

## Verify Initial State

Before running a payment, the recipient should have no open channels:

```bash
curl -sS -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","method":"list_channels","params":[{"include_closed":false}],"id":1}' \
  http://127.0.0.1:8827 | jq .
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

```bash
scripts/codespaces-demo-pay.sh 1000000000
```

`1000000000` shannons is `10 CKB`.

Successful completion prints:

```text
COMPLETED | recipient paid ...; Fiber invoice settled; LSP fee earned ...
```

The same payment flow can be run from the browser dashboard.

## Verify Final State

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

## Operating Notes

Codespaces can stop after inactivity. To restore the live stack, start the Codespace and run:

```bash
scripts/codespaces-demo-start.sh
```

If a completely clean first-receive state is needed after a payment has already been run, create a fresh node set, fund the new addresses, and start the stack with those node names.

All funds in this deployment flow are testnet-only.

## Repeated Demo Runs

The startup script opens or reuses the sender-to-LSP channel because the sender needs outbound liquidity to pay the LSP invoice. That setup channel is expected and does not invalidate the receive-first claim.

The recipient-side channel is different. On a fresh first-receive demo, the recipient should start with:

```text
recipient list_channels => []
```

After a successful payment, the recipient will have a `ChannelReady` one-way channel and non-zero local balance. Running another payment with the same recipient demonstrates reuse of existing recipient liquidity, not the zero-channel first-receive path.

For another first-receive demonstration, create and fund a new recipient node or a full fresh node set. The relevant values are runtime state and Fiber channel state, not only whether the Codespace has been restarted.

## Production Direction

The Codespaces deployment is a testnet demonstration of the architecture. A production deployment would split responsibilities more carefully:

- a long-running LSP Fiber node with managed liquidity;
- persistent order storage;
- authenticated wallet/merchant API access;
- operational monitoring for channel funding, settlement, and failed payments;
- policy controls for fees, reserves, min/max payments, and liquidity exposure.

As Fiber exposes more protocol-level hooks, this demo can evolve from RPC polling into a more native LSP service. Native payment interception or hold-payment event subscriptions would let the daemon react to payment demand directly instead of polling invoices and channels.
