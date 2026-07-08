use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Created,
    AwaitingPayment,
    PaymentHeld,
    OpeningChannel,
    ChannelReady,
    Settling,
    Completed,
    CancellingInvoice,
    Cancelled,
    Failed,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "CREATED",
            Self::AwaitingPayment => "AWAITING_PAYMENT",
            Self::PaymentHeld => "PAYMENT_HELD",
            Self::OpeningChannel => "OPENING_CHANNEL",
            Self::ChannelReady => "CHANNEL_READY",
            Self::Settling => "SETTLING",
            Self::Completed => "COMPLETED",
            Self::CancellingInvoice => "CANCELLING_INVOICE",
            Self::Cancelled => "CANCELLED",
            Self::Failed => "FAILED",
        }
    }

    pub fn transition_to(&self, next: OrderStatus) -> Result<()> {
        if self.can_transition_to(&next) {
            return Ok(());
        }

        Err(Error::InvalidTransition {
            from: self.as_str().to_string(),
            to: next.as_str().to_string(),
            reason: "transition is not allowed by the LSP order state machine".to_string(),
        })
    }

    pub fn can_transition_to(&self, next: &OrderStatus) -> bool {
        use OrderStatus::*;

        matches!(
            (self, next),
            (Created, AwaitingPayment)
                | (AwaitingPayment, PaymentHeld)
                | (AwaitingPayment, Cancelled)
                | (AwaitingPayment, Failed)
                | (PaymentHeld, OpeningChannel)
                | (PaymentHeld, CancellingInvoice)
                | (PaymentHeld, Failed)
                | (OpeningChannel, ChannelReady)
                | (OpeningChannel, CancellingInvoice)
                | (OpeningChannel, Failed)
                | (ChannelReady, Settling)
                | (ChannelReady, Failed)
                | (Settling, Completed)
                | (Settling, Failed)
                | (CancellingInvoice, Cancelled)
                | (CancellingInvoice, Failed)
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_happy_path_transitions() {
        let path = [
            OrderStatus::Created,
            OrderStatus::AwaitingPayment,
            OrderStatus::PaymentHeld,
            OrderStatus::OpeningChannel,
            OrderStatus::ChannelReady,
            OrderStatus::Settling,
            OrderStatus::Completed,
        ];

        for window in path.windows(2) {
            assert!(window[0].can_transition_to(&window[1]));
        }
    }

    #[test]
    fn rejects_settlement_before_channel_ready() {
        assert!(!OrderStatus::PaymentHeld.can_transition_to(&OrderStatus::Settling));
        assert!(!OrderStatus::OpeningChannel.can_transition_to(&OrderStatus::Settling));
    }

    #[test]
    fn rejects_transitions_out_of_terminal_states() {
        assert!(!OrderStatus::Completed.can_transition_to(&OrderStatus::Failed));
        assert!(!OrderStatus::Cancelled.can_transition_to(&OrderStatus::AwaitingPayment));
        assert!(!OrderStatus::Failed.can_transition_to(&OrderStatus::AwaitingPayment));
    }

    #[test]
    fn serializes_as_submission_state_names() {
        assert_eq!(
            serde_json::to_string(&OrderStatus::OpeningChannel).unwrap(),
            "\"OPENING_CHANNEL\""
        );
    }
}
