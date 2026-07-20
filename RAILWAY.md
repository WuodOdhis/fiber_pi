# Railway Deployment

Railway exposes one public HTTP port through the `PORT` environment variable. A service must bind to `0.0.0.0:$PORT`; binding to `127.0.0.1` only works inside the container and will not be reachable through Railway's public URL.

If the log shows this:

```text
LSP API listening listen_addr=127.0.0.1:3001
```

then the service is running locally inside the container, but Railway cannot route public traffic to it.

## Recommended Reviewer Demo

For the hackathon reviewer, deploy the full demo as one Railway service. The service runs three local Fiber nodes, `lspd`, and the browser UI in the same container. Railway exposes only the UI.

The repository includes `railway.json` and `nixpacks.toml`, so Railway should pick up the build and start commands automatically. If Railway asks for them manually, use this build command:

```bash
scripts/railway-build.sh
```

Use this start command:

```bash
scripts/railway-demo-start.sh
```

Set these variables:

```text
RUST_LOG=info
DEMO_AMOUNT=1000000000
```

Attach a Railway volume mounted at:

```text
/app/runtime
```

On the first deploy, the service prints the three CKB testnet funding addresses in Railway logs. Fund them, then redeploy/restart the service.

Suggested testnet funding:

```text
railway-sender:    500 CKB+
railway-lsp:       500 CKB+
railway-recipient: 221 CKB
```

After funding, the Railway public URL opens the demo UI. The reviewer can click `Run demo payment` and inspect the before/after recipient channel state.

## Deploy `lspd` Only

Use this Railway start command:

```bash
cargo run --release -p lspd
```

Set these variables:

```text
FIBER_RPC_URL=http://127.0.0.1:8727
RUST_LOG=info
```

Do not set `LSP_LISTEN_ADDR` unless you need to override the bind address. When `LSP_LISTEN_ADDR` is not set, `lspd` will use Railway's `PORT` automatically and bind to:

```text
0.0.0.0:$PORT
```

If you do set it manually, use this shape:

```text
LSP_LISTEN_ADDR=0.0.0.0:${PORT}
```

Railway may not expand `${PORT}` inside variable values in every context, so the automatic behavior is safer.

## Deploy The Demo UI

The demo UI also supports Railway's `PORT` variable. If deploying the UI as a separate Railway service, use:

```bash
cd demo-ui && npm install && npm start
```

Set:

```text
DEMO_UI_HOST=0.0.0.0
LSPD_URL=<internal-or-public-lspd-url>
SENDER_FIBER_URL=<sender-fiber-rpc-url>
LSP_FIBER_URL=<lsp-fiber-rpc-url>
RECIPIENT_FIBER_URL=<recipient-fiber-rpc-url>
RECIPIENT_PUBKEY=<recipient-fiber-pubkey>
RECIPIENT_ADDRESS=<recipient-p2p-address>
DEMO_AMOUNT=1000000000
```

## Important Demo Constraint

The full live demo is more than a single HTTP app. It needs three Fiber nodes, `lspd`, the UI, testnet-funded runtime keys, and persistent runtime directories. Codespaces currently matches that shape better because the demo scripts can run all local processes together.

On Railway, prefer one of these setups:

1. Deploy only the public UI/API layer to Railway, and point it at already-running Fiber nodes.
2. Use multiple Railway services plus a persistent volume for Fiber runtime state.
3. Keep Codespaces as the canonical live demo environment and use Railway only for a public wrapper/API.

For a reviewer-facing hackathon demo, the key requirement is that the browser URL stays reachable and the Fiber node state remains clean enough to prove:

```text
recipient list_channels before payment => []
recipient list_channels after payment => ChannelReady
```
