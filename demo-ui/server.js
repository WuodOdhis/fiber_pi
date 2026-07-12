import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";

const host = process.env.DEMO_UI_HOST || "127.0.0.1";
const port = Number(process.env.DEMO_UI_PORT || 5173);

const config = {
  lspdUrl: process.env.LSPD_URL || "http://127.0.0.1:3002",
  senderUrl: process.env.SENDER_FIBER_URL || "http://127.0.0.1:8627",
  lspFiberUrl: process.env.LSP_FIBER_URL || "http://127.0.0.1:8727",
  recipientUrl: process.env.RECIPIENT_FIBER_URL || "http://127.0.0.1:8827",
  recipientPubkey: process.env.RECIPIENT_PUBKEY || "03961524ed5ac2c9798ba156cd53d7802420362e15f3aafcebbb296e400cf976a0",
  recipientAddress: process.env.RECIPIENT_ADDRESS || "/ip4/127.0.0.1/tcp/8828",
  defaultAmount: process.env.DEMO_AMOUNT || "10000000000",
};

const mime = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
};

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url || "/", `http://${host}:${port}`);
    if (url.pathname === "/api/config") return json(res, config);
    if (url.pathname === "/api/rpc") return rpcProxy(req, res);
    return staticFile(url.pathname, res);
  } catch (err) {
    json(res, { error: String(err?.message || err) }, 500);
  }
});

async function rpcProxy(req, res) {
  if (req.method !== "POST") return json(res, { error: "POST required" }, 405);
  const body = await readBody(req);
  const request = JSON.parse(body || "{}");
  const target = {
    lspd: config.lspdUrl,
    sender: config.senderUrl,
    lspFiber: config.lspFiberUrl,
    recipient: config.recipientUrl,
  }[request.target];
  if (!target) return json(res, { error: `unknown target: ${request.target}` }, 400);

  const response = await fetch(target, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(request.payload),
  });
  const text = await response.text();
  res.writeHead(response.status, { "content-type": "application/json; charset=utf-8" });
  res.end(text);
}

async function staticFile(pathname, res) {
  const clean = normalize(pathname === "/" ? "/index.html" : pathname).replace(/^\/+/, "");
  if (clean.startsWith("..")) return json(res, { error: "invalid path" }, 400);
  const file = join(process.cwd(), clean);
  let data;
  try {
    data = await readFile(file);
  } catch (err) {
    if (err?.code === "ENOENT") {
      res.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      res.end("not found");
      return;
    }
    throw err;
  }
  res.writeHead(200, { "content-type": mime[extname(file)] || "application/octet-stream" });
  res.end(data);
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let data = "";
    req.on("data", (chunk) => {
      data += chunk;
    });
    req.on("end", () => resolve(data));
    req.on("error", reject);
  });
}

function json(res, data, status = 200) {
  res.writeHead(status, { "content-type": "application/json; charset=utf-8" });
  res.end(JSON.stringify(data));
}

server.listen(port, host, () => {
  console.log(`Fiber LSP demo UI: http://${host}:${port}`);
});
