use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    AwaitingPayment,
}

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
}
