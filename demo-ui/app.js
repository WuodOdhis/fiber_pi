const state = {
  config: null,
  before: null,
  after: null,
  order: null,
  senderPayment: null,
};

const els = {
  metrics: document.querySelector("#metrics"),
  amount: document.querySelector("#amount"),
  runBtn: document.querySelector("#runBtn"),
  refreshBtn: document.querySelector("#refreshBtn"),
  orderStatus: document.querySelector("#orderStatus"),
  orderKv: document.querySelector("#orderKv"),
  balanceRows: document.querySelector("#balanceRows"),
  eventsBody: document.querySelector("#eventsBody"),
  channelsBody: document.querySelector("#channelsBody"),
  log: document.querySelector("#log"),
};

boot().catch((err) => {
  setStatus("not ready", "bad");
  log("error", err.message || String(err));
});

async function boot() {
  state.config = await fetch("/api/config").then((res) => res.json());
  els.amount.value = state.config.defaultAmount;
  await refresh();
  els.runBtn.addEventListener("click", runDemo);
  els.refreshBtn.addEventListener("click", refresh);
}

async function runDemo() {
  els.runBtn.disabled = true;
  try {
    setStatus("running", "run");
    state.before = await recipientSnapshot();
    renderBalances();
    log("snapshot", `recipient local=${formatAmount(state.before.localBalance)} remote=${formatAmount(state.before.remoteBalance)}`);

    const buy = await rpc("lspd", "buy", {
      recipient_pubkey: state.config.recipientPubkey,
      recipient_address: state.config.recipientAddress,
      amount: els.amount.value,
    });
    state.order = buy.result;
    renderOrder();
    log("buy", `order=${state.order.order_id} invoice=${short(state.order.invoice)} net=${formatAmount(state.order.net_amount)}`);

    const pay = await rpc("sender", "send_payment", [{
      invoice: state.order.invoice,
      timeout: "0x258",
      max_fee_amount: "0x77359400",
    }]);
    state.senderPayment = pay.result;
    log("sender", `payment_hash=${state.senderPayment.payment_hash} status=${state.senderPayment.status}`);

    await pollOrder(state.order.order_id, state.senderPayment.payment_hash);
    state.after = await recipientSnapshot();
    renderBalances();
    await refreshChannels();
  } catch (err) {
    setStatus("failed", "bad");
    log("error", err.message || String(err));
  } finally {
    els.runBtn.disabled = false;
  }
}

async function pollOrder(orderId, paymentHash) {
  for (let i = 0; i < 180; i++) {
    const [order, payment] = await Promise.all([
      rpc("lspd", "get_order_status", { order_id: orderId }),
      rpc("sender", "get_payment", [{ payment_hash: paymentHash }]),
    ]);
    state.order = order.result;
    state.senderPayment = payment.result;
    renderOrder();
    renderEvents();
    await refreshChannels();
    log("poll", `order=${state.order.status} invoice=${state.order.invoice_status} sender=${state.senderPayment.status}`);
    if (state.order.status === "COMPLETED") {
      setStatus("completed", "ok");
      return;
    }
    if (state.order.status === "FAILED" || state.senderPayment.status === "Failed") {
      setStatus("failed", "bad");
      return;
    }
    await sleep(2000);
  }
  setStatus("timeout", "bad");
}

async function refresh() {
  state.before ||= await recipientSnapshot();
  renderMetrics();
  renderBalances();
  await refreshChannels();
}

async function refreshChannels() {
  const channels = await rpc("recipient", "list_channels", [{ include_closed: false }]);
  renderChannels(channels.result?.channels || []);
}

async function recipientSnapshot() {
  const channels = await rpc("recipient", "list_channels", [{ include_closed: false }]);
  const channel = (channels.result?.channels || [])[0] || {};
  return {
    localBalance: channel.local_balance || "0x0",
    remoteBalance: channel.remote_balance || "0x0",
    channelId: channel.channel_id || "none",
  };
}

async function rpc(target, method, params) {
  const payload = { jsonrpc: "2.0", method, params, id: Date.now() };
  const res = await fetch("/api/rpc", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ target, payload }),
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  if (data.error) throw new Error(data.error.message || data.error);
  return data;
}

function renderMetrics() {
  const items = [
    ["Recipient", short(state.config.recipientPubkey), state.config.recipientAddress],
    ["Amount", formatAmount(els.amount.value), "gross order amount"],
    ["LSP RPC", new URL(state.config.lspFiberUrl).port, "Fiber node"],
    ["Demo API", new URL(state.config.lspdUrl).port, "lspd JSON-RPC"],
  ];
  els.metrics.innerHTML = items.map(([label, value, sub]) => `<div class="metric"><div class="label">${label}</div><div class="value">${value}</div><div class="sub">${sub}</div></div>`).join("");
}

function renderOrder() {
  const order = state.order || {};
  setStatus(order.status || "idle", order.status === "COMPLETED" ? "ok" : order.status === "FAILED" ? "bad" : "run");
  const rows = [
    ["order", order.order_id],
    ["gross", formatAmount(order.gross_amount)],
    ["fee", formatAmount(order.fee_amount)],
    ["net", formatAmount(order.net_amount)],
    ["invoice", order.invoice_status],
    ["reason", order.status_reason],
    ["hash", order.payment_hash],
  ].filter((row) => row[1]);
  els.orderKv.innerHTML = rows.map(([k, v]) => `<div class="kv-row"><span>${k}</span><span>${v}</span></div>`).join("");
}

function renderEvents() {
  const events = state.order?.events || [];
  els.eventsBody.innerHTML = events.map((event) => `<tr><td>${new Date(event.timestamp_ms).toLocaleTimeString()}</td><td>${tag(event.status)}</td><td>${event.reason}</td></tr>`).join("");
}

function renderBalances() {
  const before = state.before || {};
  const after = state.after || {};
  const rows = [
    ["local_balance", before.localBalance, after.localBalance],
    ["remote_balance", before.remoteBalance, after.remoteBalance],
    ["channel_id", before.channelId, after.channelId],
  ];
  els.balanceRows.innerHTML = rows.map(([k, a, b]) => `<tr><td>${k}</td><td>${formatMaybeAmount(a)}</td><td>${formatMaybeAmount(b)}</td></tr>`).join("");
}

function renderChannels(channels) {
  els.channelsBody.innerHTML = channels.map((channel) => `<tr><td>${short(channel.channel_id)}</td><td>${tag(channel.state?.state_name || "unknown")}</td><td>${channel.is_one_way}</td><td>${formatAmount(channel.local_balance)}</td><td>${formatAmount(channel.remote_balance)}</td><td>${channel.channel_outpoint || channel.failure_detail || "-"}</td></tr>`).join("");
}

function setStatus(text, cls) {
  els.orderStatus.className = `status ${cls || "idle"}`;
  els.orderStatus.textContent = text || "idle";
}

function log(scope, text) {
  const line = document.createElement("div");
  line.className = "log-line";
  line.innerHTML = `<strong>${scope}</strong> ${text}`;
  els.log.prepend(line);
}

function tag(text) {
  const cls = text === "COMPLETED" || text === "ChannelReady" ? "green" : text === "FAILED" ? "red" : "yellow";
  return `<span class="tag ${cls}">${text}</span>`;
}

function formatMaybeAmount(value) {
  return value?.startsWith?.("0x") || /^\d+$/.test(value || "") ? formatAmount(value) : value || "-";
}

function formatAmount(value) {
  if (!value) return "-";
  const raw = value.startsWith?.("0x") ? BigInt(value) : BigInt(value);
  const whole = raw / 100000000n;
  const frac = String(raw % 100000000n).padStart(8, "0").replace(/0+$/, "");
  return `${whole}${frac ? `.${frac}` : ""} CKB`;
}

function short(value) {
  if (!value) return "-";
  return value.length > 18 ? `${value.slice(0, 10)}...${value.slice(-6)}` : value;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
