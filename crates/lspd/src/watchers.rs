use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use crate::fee::{parse_amount, to_hex_amount};
use crate::lsp_api::AppState;
use crate::model::{
    ConnectPeerParams, GetInvoiceParams, GetPaymentParams, ListChannelsParams, OpenChannelParams,
    SendPaymentParams, SettleInvoiceParams,
};
use crate::order_store::now_ms;
use crate::state_machine::OrderStatus;
use crate::Result;

const CKB_CHANNEL_RESERVED_CAPACITY_SHANNONS: u128 = 9_900_000_000;
const CKB_AUTO_ACCEPT_MIN_FUNDING_SHANNONS: u128 = 10_000_000_000;
const CKB_AUTO_ACCEPT_MIN_TOTAL_FUNDING_SHANNONS: u128 =
    CKB_CHANNEL_RESERVED_CAPACITY_SHANNONS + CKB_AUTO_ACCEPT_MIN_FUNDING_SHANNONS;
const RECIPIENT_PAYMENT_TIMEOUT_SECONDS: u64 = 120;

pub fn spawn_watchers(state: AppState) {
    let invoice_state = state.clone();
    tokio::spawn(async move { invoice_watcher(invoice_state).await });

    let channel_state = state.clone();
    tokio::spawn(async move { channel_watcher(channel_state).await });

    let executor_state = state.clone();
    tokio::spawn(async move { executor_watcher(executor_state).await });

    tokio::spawn(async move { timeout_watcher(state).await });
}

async fn invoice_watcher(state: AppState) {
    loop {
        if let Err(err) = poll_invoices(&state).await {
            error!(%err, "invoice watcher poll failed");
        }
        sleep(Duration::from_millis(state.config.poll_interval_ms)).await;
    }
}

async fn channel_watcher(state: AppState) {
    loop {
        if let Err(err) = poll_channels(&state).await {
            error!(%err, "channel watcher poll failed");
        }
        sleep(Duration::from_millis(state.config.poll_interval_ms)).await;
    }
}

async fn executor_watcher(state: AppState) {
    loop {
        if let Err(err) = poll_executions(&state).await {
            error!(%err, "executor watcher poll failed");
        }
        sleep(Duration::from_millis(state.config.poll_interval_ms)).await;
    }
}

async fn timeout_watcher(state: AppState) {
    loop {
        if let Err(err) = poll_timeouts(&state) {
            error!(%err, "timeout watcher poll failed");
        }
        sleep(Duration::from_millis(state.config.poll_interval_ms)).await;
    }
}

async fn poll_invoices(state: &AppState) -> Result<()> {
    for order in state.orders.list_active()? {
        if order.status != OrderStatus::AwaitingPayment {
            continue;
        }

        let invoice = state
            .fiber
            .get_invoice(GetInvoiceParams {
                payment_hash: order.payment_hash.clone(),
            })
            .await?;
        let Some(status) = invoice.status.as_deref() else {
            continue;
        };

        match status {
            "Open" => debug!(order_id = %order.order_id, "invoice still open"),
            "Received" => {
                info!(order_id = %order.order_id, "invoice payment received and held");
                state.orders.transition(
                    &order.order_id,
                    OrderStatus::PaymentHeld,
                    "Fiber invoice status changed to Received",
                )?;
            }
            "Cancelled" => {
                warn!(order_id = %order.order_id, "invoice cancelled");
                state.orders.transition(
                    &order.order_id,
                    OrderStatus::Cancelled,
                    "Fiber invoice was cancelled",
                )?;
            }
            "Expired" => {
                warn!(order_id = %order.order_id, "invoice expired");
                state.orders.transition(
                    &order.order_id,
                    OrderStatus::Cancelled,
                    "Fiber invoice expired",
                )?;
            }
            "Paid" => {
                warn!(order_id = %order.order_id, "invoice paid before LSP channel flow completed");
                state.orders.transition(
                    &order.order_id,
                    OrderStatus::Failed,
                    "Fiber invoice became Paid before channel provisioning completed",
                )?;
            }
            other => {
                warn!(order_id = %order.order_id, invoice_status = other, "unknown invoice status")
            }
        }
    }

    Ok(())
}

async fn poll_channels(state: &AppState) -> Result<()> {
    for order in state.orders.list_active()? {
        if order.status != OrderStatus::OpeningChannel {
            continue;
        }

        let channels = state
            .fiber
            .list_channels(ListChannelsParams {
                pubkey: Some(order.recipient_pubkey.clone()),
                include_closed: Some(true),
                only_pending: None,
            })
            .await?;

        let mut failed_reason = None;
        let mut has_live_opening = false;

        for channel in channels
            .channels
            .into_iter()
            .filter(|channel| channel_created_at_ms(&channel.created_at) >= order.updated_at_ms)
        {
            match channel.state.state_name.as_str() {
                "ChannelReady" => {
                    info!(order_id = %order.order_id, channel_id = %channel.channel_id, "channel ready");
                    state.orders.transition(
                        &order.order_id,
                        OrderStatus::ChannelReady,
                        format!("Fiber channel {} reached ChannelReady", channel.channel_id),
                    )?;
                    break;
                }
                _ if channel.failure_detail.is_some() => {
                    failed_reason = Some(
                        channel
                            .failure_detail
                            .unwrap_or_else(|| "channel closed during opening".to_string()),
                    );
                }
                state_name => {
                    has_live_opening = true;
                    debug!(order_id = %order.order_id, state_name, "channel not ready yet")
                }
            }
        }

        let latest_order = state.orders.get(&order.order_id)?;
        if latest_order.status == OrderStatus::OpeningChannel && !has_live_opening {
            if let Some(reason) = failed_reason {
                warn!(order_id = %order.order_id, %reason, "channel opening failed");
                state
                    .orders
                    .transition(&order.order_id, OrderStatus::Failed, reason)?;
            }
        }
    }

    Ok(())
}

fn channel_created_at_ms(value: &str) -> u64 {
    value
        .strip_prefix("0x")
        .and_then(|hex| u64::from_str_radix(hex, 16).ok())
        .unwrap_or_default()
}

async fn poll_executions(state: &AppState) -> Result<()> {
    for order in state.orders.list_active()? {
        match order.status {
            OrderStatus::PaymentHeld => start_channel_open(state, &order).await?,
            OrderStatus::ChannelReady => settle_order(state, &order).await?,
            _ => {}
        }
    }

    Ok(())
}

async fn start_channel_open(state: &AppState, order: &crate::order_store::Order) -> Result<()> {
    state.orders.transition(
        &order.order_id,
        OrderStatus::OpeningChannel,
        "starting Fiber channel open to recipient",
    )?;

    let net_amount = parse_amount(&order.net_amount)?;
    if has_ready_recipient_capacity(state, order, net_amount).await? {
        state.orders.transition(
            &order.order_id,
            OrderStatus::ChannelReady,
            "existing Fiber channel has enough recipient capacity",
        )?;
        return Ok(());
    }

    if let Err(err) = ensure_recipient_connected(state, order).await {
        warn!(order_id = %order.order_id, %err, "failed to connect recipient before channel open");
        state.orders.transition(
            &order.order_id,
            OrderStatus::Failed,
            format!("failed to connect recipient: {err}"),
        )?;
        return Ok(());
    }

    let funding_amount = if order.currency == "Fibt" || order.currency == "Fibb" {
        net_amount
            .saturating_add(CKB_CHANNEL_RESERVED_CAPACITY_SHANNONS)
            .max(CKB_AUTO_ACCEPT_MIN_TOTAL_FUNDING_SHANNONS)
    } else {
        net_amount
    };
    match state
        .fiber
        .open_channel(OpenChannelParams {
            pubkey: order.recipient_pubkey.clone(),
            funding_amount: to_hex_amount(funding_amount),
            public: Some(false),
            one_way: Some(true),
            funding_udt_type_script: None,
            shutdown_script: None,
            commitment_delay_epoch: None,
            commitment_fee_rate: None,
            funding_fee_rate: None,
            tlc_expiry_delta: None,
            tlc_min_value: None,
            tlc_fee_proportional_millionths: None,
            max_tlc_value_in_flight: None,
            max_tlc_number_in_flight: None,
        })
        .await
    {
        Ok(opened) => {
            info!(
                order_id = %order.order_id,
                temporary_channel_id = %opened.temporary_channel_id,
                "Fiber channel open started"
            );
        }
        Err(err) => {
            warn!(order_id = %order.order_id, %err, "Fiber channel open failed");
            state.orders.transition(
                &order.order_id,
                OrderStatus::Failed,
                format!("Fiber open_channel failed: {err}"),
            )?;
        }
    }

    Ok(())
}

async fn has_ready_recipient_capacity(
    state: &AppState,
    order: &crate::order_store::Order,
    required_amount: u128,
) -> Result<bool> {
    let channels = state
        .fiber
        .list_channels(ListChannelsParams {
            pubkey: Some(order.recipient_pubkey.clone()),
            include_closed: Some(false),
            only_pending: None,
        })
        .await?;

    for channel in channels.channels {
        if channel.state.state_name == "ChannelReady"
            && parse_amount(&channel.local_balance)? >= required_amount
        {
            return Ok(true);
        }
    }

    Ok(false)
}

async fn ensure_recipient_connected(
    state: &AppState,
    order: &crate::order_store::Order,
) -> Result<()> {
    let peers = state.fiber.list_peers().await?;
    if peers
        .peers
        .iter()
        .any(|peer| peer.pubkey == order.recipient_pubkey)
    {
        return Ok(());
    }

    state
        .fiber
        .connect_peer(ConnectPeerParams {
            address: order.recipient_address.clone(),
            pubkey: Some(order.recipient_pubkey.clone()),
            save: Some(true),
            addr_type: None,
        })
        .await
}

async fn settle_order(state: &AppState, order: &crate::order_store::Order) -> Result<()> {
    state.orders.transition(
        &order.order_id,
        OrderStatus::Settling,
        "paying recipient before settling Fiber hold invoice",
    )?;

    if let Err(err) = pay_recipient(state, order).await {
        warn!(order_id = %order.order_id, %err, "recipient payment failed");
        state.orders.transition(
            &order.order_id,
            OrderStatus::Failed,
            format!("Fiber recipient payment failed: {err}"),
        )?;
        return Ok(());
    }

    match state
        .fiber
        .settle_invoice(SettleInvoiceParams {
            payment_hash: order.payment_hash.clone(),
            payment_preimage: order.payment_preimage.clone(),
        })
        .await
    {
        Ok(_) => {
            state.orders.transition(
                &order.order_id,
                OrderStatus::Completed,
                format!(
                    "recipient paid {}; Fiber invoice settled; LSP fee earned {}",
                    order.net_amount, order.fee_amount
                ),
            )?;
        }
        Err(err) => {
            warn!(order_id = %order.order_id, %err, "Fiber invoice settlement failed");
            state.orders.transition(
                &order.order_id,
                OrderStatus::Failed,
                format!("Fiber settle_invoice failed: {err}"),
            )?;
        }
    }

    Ok(())
}

async fn pay_recipient(state: &AppState, order: &crate::order_store::Order) -> Result<()> {
    let payment = state
        .fiber
        .send_payment(SendPaymentParams {
            target_pubkey: Some(order.recipient_pubkey.clone()),
            amount: Some(to_hex_amount(parse_amount(&order.net_amount)?)),
            payment_hash: None,
            final_tlc_expiry_delta: None,
            tlc_expiry_limit: None,
            invoice: None,
            timeout: Some(to_hex_amount(RECIPIENT_PAYMENT_TIMEOUT_SECONDS as u128)),
            max_fee_amount: Some("0x0".to_string()),
            max_fee_rate: None,
            max_parts: None,
            trampoline_hops: None,
            keysend: Some(true),
            udt_type_script: None,
            allow_self_payment: None,
            custom_records: None,
            hop_hints: None,
            dry_run: None,
        })
        .await?;

    wait_for_payment_success(state, &payment.payment_hash).await
}

async fn wait_for_payment_success(state: &AppState, payment_hash: &str) -> Result<()> {
    let attempts = RECIPIENT_PAYMENT_TIMEOUT_SECONDS / 2;
    for _ in 0..attempts {
        let payment = state
            .fiber
            .get_payment(GetPaymentParams {
                payment_hash: payment_hash.to_string(),
            })
            .await?;

        match payment.status.as_str() {
            "Success" => return Ok(()),
            "Failed" => {
                return Err(crate::Error::Server(format!(
                    "recipient keysend failed: {}",
                    payment
                        .failed_error
                        .unwrap_or_else(|| "unknown payment error".to_string())
                )))
            }
            _ => sleep(Duration::from_secs(2)).await,
        }
    }

    Err(crate::Error::Server(format!(
        "recipient keysend {payment_hash} did not complete before timeout"
    )))
}

fn poll_timeouts(state: &AppState) -> Result<()> {
    let now = now_ms();
    let timeout_ms = state.config.order_timeout_seconds.saturating_mul(1000);

    for order in state.orders.list_active()? {
        if now.saturating_sub(order.created_at_ms) < timeout_ms {
            continue;
        }

        match order.status {
            OrderStatus::AwaitingPayment => {
                state.orders.transition(
                    &order.order_id,
                    OrderStatus::Cancelled,
                    "order timed out while awaiting payment",
                )?;
            }
            OrderStatus::PaymentHeld | OrderStatus::OpeningChannel => {
                state.orders.transition(
                    &order.order_id,
                    OrderStatus::Failed,
                    "order timed out before channel provisioning completed",
                )?;
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::order_store::{Order, OrderStore};

    fn test_state() -> AppState {
        AppState {
            config: Config {
                fiber_rpc_url: "http://127.0.0.1:8427".to_string(),
                listen_addr: "127.0.0.1:3001".parse().unwrap(),
                fee_rate_bps: 100,
                min_amount: 1,
                max_amount: 1_000_000,
                currency: "Fibt".to_string(),
                invoice_expiry_seconds: 3600,
                poll_interval_ms: 10,
                order_timeout_seconds: 1,
            },
            fiber: crate::FiberRpcClient::new("http://127.0.0.1:8427"),
            orders: OrderStore::default(),
        }
    }

    fn test_order(status: OrderStatus, created_at_ms: u64) -> Order {
        Order {
            order_id: "order-1".to_string(),
            recipient_pubkey: "recipient".to_string(),
            recipient_address: None,
            invoice: "invoice".to_string(),
            payment_hash: "0xhash".to_string(),
            payment_preimage: "0xpreimage".to_string(),
            gross_amount: "1000".to_string(),
            fee_amount: "10".to_string(),
            net_amount: "990".to_string(),
            currency: "Fibt".to_string(),
            status: status.clone(),
            status_reason: None,
            created_at_ms,
            updated_at_ms: created_at_ms,
            events: vec![crate::order_store::initial_event(
                status,
                "test order created",
                created_at_ms,
            )],
        }
    }

    #[test]
    fn timeout_cancels_order_awaiting_payment() {
        let state = test_state();
        state
            .orders
            .insert(test_order(OrderStatus::AwaitingPayment, 1))
            .unwrap();

        poll_timeouts(&state).unwrap();

        let order = state.orders.get("order-1").unwrap();
        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[test]
    fn timeout_fails_payment_held_order() {
        let state = test_state();
        state
            .orders
            .insert(test_order(OrderStatus::PaymentHeld, 1))
            .unwrap();

        poll_timeouts(&state).unwrap();

        let order = state.orders.get("order-1").unwrap();
        assert_eq!(order.status, OrderStatus::Failed);
    }
}
