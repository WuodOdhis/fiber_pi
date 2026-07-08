use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use tracing::{debug, instrument};

use crate::error::{Error, Result};
use crate::model::{
    ConnectPeerParams, GetInvoiceParams, InvoiceResult, ListChannelsParams, ListChannelsResult,
    ListPeersResult, NewInvoiceParams, NodeInfo, OpenChannelParams, OpenChannelResult,
    SettleInvoiceParams,
};

#[derive(Debug, Clone)]
pub struct FiberRpcClient {
    url: String,
    http: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'static str,
    method: &'a str,
    params: Value,
    id: u64,
}

#[derive(Debug, serde::Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, serde::Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

impl FiberRpcClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    #[instrument(skip(self), fields(url = %self.url))]
    pub async fn node_info(&self) -> Result<NodeInfo> {
        self.call_no_params("node_info").await
    }

    #[instrument(skip(self, params), fields(url = %self.url, pubkey = ?params.pubkey, address = ?params.address))]
    pub async fn connect_peer(&self, params: ConnectPeerParams) -> Result<()> {
        self.call_one("connect_peer", params).await
    }

    #[instrument(skip(self), fields(url = %self.url))]
    pub async fn list_peers(&self) -> Result<ListPeersResult> {
        self.call_no_params("list_peers").await
    }

    #[instrument(skip(self, params), fields(url = %self.url, payment_hash = ?params.payment_hash))]
    pub async fn new_invoice(&self, params: NewInvoiceParams) -> Result<InvoiceResult> {
        self.call_one("new_invoice", params).await
    }

    #[instrument(skip(self, params), fields(url = %self.url, payment_hash = %params.payment_hash))]
    pub async fn get_invoice(&self, params: GetInvoiceParams) -> Result<InvoiceResult> {
        self.call_one("get_invoice", params).await
    }

    #[instrument(skip(self, params), fields(url = %self.url, payment_hash = %params.payment_hash))]
    pub async fn settle_invoice(&self, params: SettleInvoiceParams) -> Result<()> {
        self.call_one("settle_invoice", params).await
    }

    #[instrument(skip(self, params), fields(url = %self.url, pubkey = %params.pubkey))]
    pub async fn open_channel(&self, params: OpenChannelParams) -> Result<OpenChannelResult> {
        self.call_one("open_channel", params).await
    }

    #[instrument(skip(self, params), fields(url = %self.url))]
    pub async fn list_channels(&self, params: ListChannelsParams) -> Result<ListChannelsResult> {
        self.call_one("list_channels", params).await
    }

    async fn call_no_params<T>(&self, method: &'static str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.call(method, json!([])).await
    }

    async fn call_one<P, T>(&self, method: &'static str, params: P) -> Result<T>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        self.call(method, json!([params])).await
    }

    async fn call<T>(&self, method: &'static str, params: Value) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            method,
            params,
            id: 1,
        };

        debug!(method, "calling Fiber RPC");
        let response = self.http.post(&self.url).json(&request).send().await?;
        let body = response.error_for_status()?.text().await?;
        debug!(method, body, "received Fiber RPC response");

        let rpc_response: JsonRpcResponse<T> = serde_json::from_str(&body)?;
        if let Some(error) = rpc_response.error {
            return Err(Error::Rpc {
                method,
                code: error.code,
                message: error.message,
                data: error.data,
            });
        }

        rpc_response.result.ok_or(Error::MissingResult { method })
    }
}

#[cfg(test)]
mod tests {
    use crate::model::NewInvoiceParams;

    #[test]
    fn serializes_structured_params_as_single_object_array() {
        let params = NewInvoiceParams {
            amount: "0x174876e800".to_string(),
            currency: "Fibt".to_string(),
            description: Some("test".to_string()),
            payment_preimage: None,
            payment_hash: Some("0xabc".to_string()),
            expiry: Some("0xe10".to_string()),
            fallback_address: None,
            final_expiry_delta: None,
            udt_type_script: None,
            hash_algorithm: Some("sha256".to_string()),
            allow_mpp: None,
            allow_trampoline_routing: None,
        };

        let serialized = serde_json::to_value(vec![params]).unwrap();
        assert!(serialized.is_array());
        assert_eq!(serialized[0]["currency"], "Fibt");
        assert_eq!(serialized[0]["hash_algorithm"], "sha256");
        assert!(serialized[0].get("payment_preimage").is_none());
    }
}
