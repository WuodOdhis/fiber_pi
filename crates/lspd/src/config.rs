use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub fiber_rpc_url: String,
    pub listen_addr: SocketAddr,
    pub fee_rate_bps: u128,
    pub min_amount: u128,
    pub max_amount: u128,
    pub currency: String,
    pub invoice_expiry_seconds: u64,
    pub poll_interval_ms: u64,
    pub order_timeout_seconds: u64,
}

impl Config {
    pub fn from_env() -> crate::Result<Self> {
        let fiber_rpc_url =
            std::env::var("FIBER_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8427".to_string());
        let listen_addr = listen_addr_from_env()?;

        Ok(Self {
            fiber_rpc_url,
            listen_addr,
            fee_rate_bps: read_u128("FEE_RATE_BPS", 100)?,
            min_amount: read_u128("MIN_AMOUNT", 100_000_000)?,
            max_amount: read_u128("MAX_AMOUNT", 10_000_000_000_000)?,
            currency: std::env::var("FIBER_CURRENCY").unwrap_or_else(|_| "Fibt".to_string()),
            invoice_expiry_seconds: read_u64("INVOICE_EXPIRY_SECONDS", 3600)?,
            poll_interval_ms: read_u64("POLL_INTERVAL_MS", 2000)?,
            order_timeout_seconds: read_u64("ORDER_TIMEOUT_SECONDS", 7200)?,
        })
    }
}

fn listen_addr_from_env() -> crate::Result<SocketAddr> {
    let value = match std::env::var("LSP_LISTEN_ADDR") {
        Ok(value) => value,
        Err(_) => match std::env::var("PORT") {
            Ok(port) => format!("0.0.0.0:{port}"),
            Err(_) => "127.0.0.1:3001".to_string(),
        },
    };

    value
        .parse()
        .map_err(|err| crate::Error::Server(format!("invalid LSP_LISTEN_ADDR/PORT: {err}")))
}

fn read_u128(name: &'static str, default: u128) -> crate::Result<u128> {
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|err| crate::Error::Server(format!("invalid {name}: {err}"))),
        Err(_) => Ok(default),
    }
}

fn read_u64(name: &'static str, default: u64) -> crate::Result<u64> {
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|err| crate::Error::Server(format!("invalid {name}: {err}"))),
        Err(_) => Ok(default),
    }
}
