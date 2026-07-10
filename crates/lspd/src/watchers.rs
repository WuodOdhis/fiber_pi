use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use crate::lsp_api::AppState;
use crate::model::{GetInvoiceParams, ListChannelsParams};
use crate::order_store::now_ms;
use crate::state_machine::OrderStatus;
use crate::Result;

pub fn spawn_watchers(state: AppState) {
    let invoice_state = state.clone();
    tokio::spawn(async move { invoice_watcher(invoice_state).await });

    let channel_state = state.clone();
    tokio::spawn(async move { channel_watcher(channel_state).await });

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
