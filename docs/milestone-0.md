# Milestone 0: Fiber Environment Verification

Goal: prove the project can run multiple Fiber nodes locally and talk to them through RPC before building LSP daemon logic.

## Current Findings

- Official Fiber repo: `https://github.com/nervosnetwork/fiber`
- Pinned Fiber version for this project: `v0.8.1`
- Stable release observed: `v0.8.1`
- Newer pre-release observed: `v0.9.0-rc7`
- Required Rust toolchain from Fiber repo: `1.93.0`
- Local machine has Rust/Cargo and Docker available.
- `offckb` is installed.
- `ckb` and `ckb-cli` are not currently available on PATH.
- Published Docker images were not pullable from this environment for `nervos/fiber:v0.8.1`, `ghcr.io/nervosnetwork/fiber:v0.8.1`, or `latest`.
- Local source build of `fnn` and `fnn-cli` from `v0.8.1` succeeded.
- Three local `fnn` processes start successfully with generated demo keys.
- All three local RPC endpoints respond to `fnn-cli info`.
- Direct local peer connections are verified: sender connects to LSP, and LSP connects to recipient.

## Important Constraint

The official Fiber `config/testnet/config.yml` points to public CKB testnet and includes deployed Fiber script dependencies. A pure local CKB devnet is not automatically equivalent unless Fiber scripts are deployed to that devnet.

For Milestone 0, the practical environment is:

```text
three local fnn processes
official Fiber testnet script configuration
public CKB testnet RPC from Fiber config unless overridden
```

This still verifies the local Fiber node topology and RPC surface. Local CKB devnet integration remains a separate verification item.

## Acceptance Criteria

Milestone 0 is complete only when all of these are true:

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

Full payment/channel tests belong to Milestone 1 after RPC shapes are captured.

## Demo Node Ports

```text
sender:    p2p 8328, rpc 8327
lsp:       p2p 8428, rpc 8427
recipient: p2p 8528, rpc 8527
```

## Scripts

Run from the project root:

```bash
scripts/check-env.sh
scripts/prepare-fiber.sh
scripts/init-demo-nodes.sh
scripts/probe-demo-nodes.sh
```

Start nodes in separate terminals:

```bash
scripts/start-node.sh sender
scripts/start-node.sh lsp
scripts/start-node.sh recipient
```

Check node info:

```bash
.fiber-bin/fnn-cli -u http://127.0.0.1:8327 info
.fiber-bin/fnn-cli -u http://127.0.0.1:8427 info
.fiber-bin/fnn-cli -u http://127.0.0.1:8527 info
```

Connect local peers while all three nodes are running:

```bash
scripts/connect-demo-peers.sh
```

## Open Items Before Milestone 1

1. Determine the address/funding path for each node key.
2. Decide whether to use public CKB testnet funding or deploy Fiber scripts to an OffCKB devnet.
3. Capture actual `node_info`, `connect_peer`, `new_invoice`, `get_invoice`, `open_channel`, `list_channels`, and `settle_invoice` payloads.
