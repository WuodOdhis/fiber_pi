use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::state_machine::OrderStatus;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: String,
    pub recipient_pubkey: String,
    pub recipient_address: Option<String>,
    pub invoice: String,
    pub payment_hash: String,
    #[serde(skip_serializing)]
    pub payment_preimage: String,
    pub gross_amount: String,
    pub fee_amount: String,
    pub net_amount: String,
    pub currency: String,
    pub status: OrderStatus,
    #[serde(default)]
    pub status_reason: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct OrderStore {
    orders: Arc<Mutex<HashMap<String, Order>>>,
}

impl OrderStore {
    pub fn insert(&self, order: Order) -> Result<()> {
        let mut orders = self
            .orders
            .lock()
            .map_err(|_| Error::Server("order store lock poisoned".to_string()))?;
        orders.insert(order.order_id.clone(), order);
        Ok(())
    }

    pub fn get(&self, order_id: &str) -> Result<Order> {
        let orders = self
            .orders
            .lock()
            .map_err(|_| Error::Server("order store lock poisoned".to_string()))?;
        orders
            .get(order_id)
            .cloned()
            .ok_or_else(|| Error::OrderNotFound(order_id.to_string()))
    }

    pub fn list_active(&self) -> Result<Vec<Order>> {
        let orders = self
            .orders
            .lock()
            .map_err(|_| Error::Server("order store lock poisoned".to_string()))?;
        Ok(orders
            .values()
            .filter(|order| !order.status.is_terminal())
            .cloned()
            .collect())
    }

    pub fn transition(
        &self,
        order_id: &str,
        next_status: OrderStatus,
        reason: impl Into<String>,
    ) -> Result<Order> {
        let mut orders = self
            .orders
            .lock()
            .map_err(|_| Error::Server("order store lock poisoned".to_string()))?;
        let order = orders
            .get_mut(order_id)
            .ok_or_else(|| Error::OrderNotFound(order_id.to_string()))?;

        order.status.transition_to(next_status.clone())?;
        order.status = next_status;
        order.status_reason = Some(reason.into());
        order.updated_at_ms = now_ms();
        Ok(order.clone())
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_order(status: OrderStatus) -> Order {
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
            status,
            status_reason: None,
            created_at_ms: 1,
            updated_at_ms: 1,
        }
    }

    #[test]
    fn store_applies_valid_transition() {
        let store = OrderStore::default();
        store
            .insert(test_order(OrderStatus::AwaitingPayment))
            .unwrap();

        let updated = store
            .transition("order-1", OrderStatus::PaymentHeld, "invoice held")
            .unwrap();

        assert_eq!(updated.status, OrderStatus::PaymentHeld);
        assert_eq!(updated.status_reason.as_deref(), Some("invoice held"));
    }

    #[test]
    fn store_rejects_invalid_transition() {
        let store = OrderStore::default();
        store
            .insert(test_order(OrderStatus::AwaitingPayment))
            .unwrap();

        let err = store
            .transition("order-1", OrderStatus::Completed, "skip ahead")
            .unwrap_err();

        assert!(matches!(err, Error::InvalidTransition { .. }));
    }

    #[test]
    fn list_active_excludes_terminal_orders() {
        let store = OrderStore::default();
        store
            .insert(test_order(OrderStatus::AwaitingPayment))
            .unwrap();

        let mut completed = test_order(OrderStatus::Completed);
        completed.order_id = "order-2".to_string();
        store.insert(completed).unwrap();

        let active = store.list_active().unwrap();

        assert_eq!(active.len(), 1);
        assert_eq!(active[0].order_id, "order-1");
    }
}
