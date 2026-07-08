use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Fiber RPC error from {method}: code {code}, message: {message}")]
    Rpc {
        method: &'static str,
        code: i64,
        message: String,
        data: Option<serde_json::Value>,
    },

    #[error("Fiber RPC response for {method} did not include result or error")]
    MissingResult { method: &'static str },

    #[error("invalid amount: {0}")]
    InvalidAmount(String),

    #[error("order not found: {0}")]
    OrderNotFound(String),

    #[error("invalid order transition from {from} to {to}: {reason}")]
    InvalidTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("server error: {0}")]
    Server(String),
}
