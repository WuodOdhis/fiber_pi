# Demo Runbook

This runbook is Milestone 0 only. It verifies Fiber node setup before building LSP daemon logic.

## 1. Check Environment

```bash
scripts/check-env.sh
```

Expected tools:

```text
git
cargo
rustc
docker optional
offckb optional
```

## 2. Prepare Fiber Binaries

```bash
scripts/prepare-fiber.sh
```

This clones Fiber `v0.8.1` into `.fiber-src/fiber-v0.8.1`, builds `fnn` and `fnn-cli`, then copies binaries into `.fiber-bin/`.

## 3. Initialize Demo Nodes

```bash
scripts/init-demo-nodes.sh
```

This creates:

```text
runtime/sender/config.yml
runtime/sender/ckb/key
runtime/lsp/config.yml
runtime/lsp/ckb/key
runtime/recipient/config.yml
runtime/recipient/ckb/key
```

The generated configs use unique local P2P and RPC ports.

## 4. Start Nodes

Use three terminals:

```bash
scripts/start-node.sh sender
scripts/start-node.sh lsp
scripts/start-node.sh recipient
```

## 5. Query Nodes

```bash
.fiber-bin/fnn-cli -u http://127.0.0.1:8327 info
.fiber-bin/fnn-cli -u http://127.0.0.1:8427 info
.fiber-bin/fnn-cli -u http://127.0.0.1:8527 info
```

## 6. Peer Connections

After `info` works for all nodes, capture each node's public key and address from `fnn-cli info`.

Then connect using the helper script:

```bash
scripts/connect-demo-peers.sh
```

Equivalent manual commands:

```bash
.fiber-bin/fnn-cli -u http://127.0.0.1:8327 peer connect_peer --address /ip4/127.0.0.1/tcp/8428 --pubkey <lsp-pubkey> --save true
.fiber-bin/fnn-cli -u http://127.0.0.1:8427 peer connect_peer --address /ip4/127.0.0.1/tcp/8528 --pubkey <recipient-pubkey> --save true
```

Do not continue to daemon implementation until these commands are verified against real node output.

Verified local topology:

```text
sender -> lsp
lsp -> recipient
```
