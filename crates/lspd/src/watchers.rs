use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use crate::fee::{parse_amount, to_hex_amount};
use crate::lsp_api::AppState;
use crate::model::{
    ConnectPeerParams, GetInvoiceParams, ListChannelsParams, OpenChannelParams, SettleInvoiceParams,
};
use crate::order_store::now_ms;
use crate::state_machine::OrderStatus;
use crate::Result;

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
                include_closed: None,
                only_pending: Some(true),
            })
            .await?;

        for channel in channels.channels {
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
                "Closed" if channel.failure_detail.is_some() => {
                    let reason = channel
                        .failure_detail
                        .unwrap_or_else(|| "channel closed during opening".to_string());
                    warn!(order_id = %order.order_id, %reason, "channel opening failed");
                    state
                        .orders
                        .transition(&order.order_id, OrderStatus::Failed, reason)?;
                    break;
                }
                state_name => {
                    debug!(order_id = %order.order_id, state_name, "channel not ready yet")
                }
            }
        }
    }

    Ok(())
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

    if let Err(err) = ensure_recipient_connected(state, order).await {
        warn!(order_id = %order.order_id, %err, "failed to connect recipient before channel open");
        state.orders.transition(
            &order.order_id,
            OrderStatus::Failed,
            format!("failed to connect recipient: {err}"),
        )?;
        return Ok(());
    }

    let net_amount = parse_amount(&order.net_amount)?;
    match state
        .fiber
        .open_channel(OpenChannelParams {
            pubkey: order.recipient_pubkey.clone(),
            funding_amount: to_hex_amount(net_amount),
            public: Some(false),
            one_way: Some(false),
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
        "settling Fiber hold invoice after channel readiness",
    )?;

    match state
        .fiber
        .settle_invoice(SettleInvoiceParams {
            payment_hash: order.payment_hash.clone(),
            payment_preimage: order.payment_preimage.clone(),
        })
        .await
    {
        Ok(()) => {
            state.orders.transition(
                &order.order_id,
                OrderStatus::Completed,
                format!("Fiber invoice settled; LSP fee earned {}", order.fee_amount),
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
