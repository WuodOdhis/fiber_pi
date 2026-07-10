use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::config::Config;
use crate::fee::{calculate_fee, parse_amount, to_hex_amount};
use crate::model::{GetInvoiceParams, ListChannelsParams, NewInvoiceParams};
use crate::order_store::{initial_event, now_ms, Order, OrderStore};
use crate::state_machine::OrderStatus;
use crate::watchers::spawn_watchers;
use crate::{FiberRpcClient, Result};

#[derive(Debug, Clone)]
pub struct AppState {
    pub(crate) config: Config,
    pub(crate) fiber: FiberRpcClient,
    pub(crate) orders: OrderStore,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let fiber = FiberRpcClient::new(config.fiber_rpc_url.clone());
        Self {
            config,
            fiber,
            orders: OrderStore::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    method: String,
    #[serde(default)]
    params: Value,
    #[serde(default)]
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Deserialize)]
struct BuyParams {
    recipient_pubkey: String,
    #[serde(default)]
    recipient_address: Option<String>,
    amount: String,
}

#[derive(Debug, Deserialize)]
struct GetOrderStatusParams {
    order_id: String,
}

pub async fn serve(config: Config) -> Result<()> {
    let listen_addr = config.listen_addr;
    let state = AppState::new(config);
    spawn_watchers(state.clone());

    let app = Router::new().route("/", post(handle_rpc)).with_state(state);

    let listener = TcpListener::bind(listen_addr).await.map_err(|err| {
        crate::Error::Server(format!("failed to bind LSP API at {listen_addr}: {err}"))
    })?;

    info!(%listen_addr, "LSP API listening");
    axum::serve(listener, app)
        .await
        .map_err(|err| crate::Error::Server(format!("LSP API server failed: {err}")))
}

async fn handle_rpc(
    State(state): State<AppState>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let id = request.id.clone();
    let result = match request.method.as_str() {
        "get_info" => get_info(&state).await,
        "buy" => buy(&state, request.params).await,
        "get_order_status" => get_order_status(&state, request.params).await,
        method => Err(crate::Error::Server(format!("unknown method: {method}"))),
    };

    match result {
        Ok(result) => Json(JsonRpcResponse {
            jsonrpc: "2.0",
            result: Some(result),
            error: None,
            id,
        }),
        Err(err) => {
            error!(%err, "LSP API request failed");
            Json(JsonRpcResponse {
                jsonrpc: "2.0",
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: err.to_string(),
                }),
                id,
            })
        }
    }
}

#[instrument(skip(state))]
async fn get_info(state: &AppState) -> Result<Value> {
    let node = state.fiber.node_info().await?;
    Ok(json!({
        "service": "fiber-lsp-daemon",
        "version": env!("CARGO_PKG_VERSION"),
        "lsp_pubkey": node.pubkey,
        "fiber_version": node.version,
        "fiber_commit_hash": node.commit_hash,
        "currency": state.config.currency,
        "fee_rate_bps": state.config.fee_rate_bps.to_string(),
        "min_amount": state.config.min_amount.to_string(),
        "max_amount": state.config.max_amount.to_string(),
        "invoice_expiry_seconds": state.config.invoice_expiry_seconds,
        "poll_interval_ms": state.config.poll_interval_ms,
        "order_timeout_seconds": state.config.order_timeout_seconds,
    }))
}

#[instrument(skip(state, params))]
async fn buy(state: &AppState, params: Value) -> Result<Value> {
    let params: BuyParams = serde_json::from_value(params)?;
    let gross_amount = parse_amount(&params.amount)?;
    if gross_amount < state.config.min_amount || gross_amount > state.config.max_amount {
        return Err(crate::Error::InvalidAmount(format!(
            "{} outside configured range {}..{}",
            params.amount, state.config.min_amount, state.config.max_amount
        )));
    }

    let fee_amount = calculate_fee(gross_amount, state.config.fee_rate_bps);
    let net_amount = gross_amount.saturating_sub(fee_amount);
    let (payment_preimage, payment_hash) = new_sha256_payment_pair();

    let invoice = state
        .fiber
        .new_invoice(NewInvoiceParams {
            amount: to_hex_amount(gross_amount),
            currency: state.config.currency.clone(),
            description: Some("Fiber LSP liquidity order".to_string()),
            payment_preimage: None,
            payment_hash: Some(payment_hash.clone()),
            expiry: Some(to_hex_amount(state.config.invoice_expiry_seconds as u128)),
            fallback_address: None,
            final_expiry_delta: None,
            udt_type_script: None,
            hash_algorithm: Some("sha256".to_string()),
            allow_mpp: None,
            allow_trampoline_routing: None,
        })
        .await?;

    let order_id = Uuid::new_v4().to_string();
    let now = now_ms();
    let initial_status = OrderStatus::AwaitingPayment;
    let initial_reason = "hold invoice created";
    let order = Order {
        order_id: order_id.clone(),
        recipient_pubkey: params.recipient_pubkey,
        recipient_address: params.recipient_address,
        invoice: invoice.invoice_address.clone(),
        payment_hash: payment_hash.clone(),
        payment_preimage,
        gross_amount: gross_amount.to_string(),
        fee_amount: fee_amount.to_string(),
        net_amount: net_amount.to_string(),
        currency: state.config.currency.clone(),
        status: initial_status.clone(),
        status_reason: Some(initial_reason.to_string()),
        created_at_ms: now,
        updated_at_ms: now,
        events: vec![initial_event(initial_status, initial_reason, now)],
    };
    state.orders.insert(order.clone())?;

    Ok(json!({
        "order_id": order_id,
        "invoice": order.invoice,
        "payment_hash": order.payment_hash,
        "gross_amount": order.gross_amount,
        "fee_amount": order.fee_amount,
        "net_amount": order.net_amount,
        "currency": order.currency,
        "status": order.status.as_str(),
        "status_reason": order.status_reason,
        "events": order.events,
        "fiber_invoice_status": invoice.status,
    }))
}

#[instrument(skip(state, params))]
async fn get_order_status(state: &AppState, params: Value) -> Result<Value> {
    let params: GetOrderStatusParams = serde_json::from_value(params)?;
    let order = state.orders.get(&params.order_id)?;
    let invoice = state
        .fiber
        .get_invoice(GetInvoiceParams {
            payment_hash: order.payment_hash.clone(),
        })
        .await?;
    let channels = state
        .fiber
        .list_channels(ListChannelsParams {
            pubkey: Some(order.recipient_pubkey.clone()),
            include_closed: Some(true),
            only_pending: None,
        })
        .await?;
    let channel_summaries = channels
        .channels
        .into_iter()
        .map(|channel| {
            json!({
                "channel_id": channel.channel_id,
                "state": channel.state.state_name,
                "state_flags": channel.state.state_flags,
                "local_balance": channel.local_balance,
                "remote_balance": channel.remote_balance,
                "failure_detail": channel.failure_detail,
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "order_id": order.order_id,
        "status": order.status.as_str(),
        "status_reason": order.status_reason,
        "invoice_status": invoice.status,
        "created_at_ms": order.created_at_ms,
        "updated_at_ms": order.updated_at_ms,
        "events": order.events,
        "channels": channel_summaries,
        "payment_hash": order.payment_hash,
        "gross_amount": order.gross_amount,
        "fee_amount": order.fee_amount,
        "net_amount": order.net_amount,
        "currency": order.currency,
    }))
}

fn new_sha256_payment_pair() -> (String, String) {
    let mut preimage = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut preimage);
    let hash = Sha256::digest(preimage);
    (
        format!("0x{}", hex::encode(preimage)),
        format!("0x{}", hex::encode(hash)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_payment_pair_uses_hex_32_byte_values() {
        let (preimage, hash) = new_sha256_payment_pair();
        assert_eq!(preimage.len(), 66);
        assert_eq!(hash.len(), 66);
        assert!(preimage.starts_with("0x"));
        assert!(hash.starts_with("0x"));
    }
}
