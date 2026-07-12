use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Script {
    pub code_hash: String,
    pub hash_type: String,
    pub args: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeInfo {
    pub version: String,
    pub commit_hash: String,
    pub pubkey: String,
    pub features: Vec<String>,
    pub node_name: Option<String>,
    pub addresses: Vec<String>,
    pub chain_hash: String,
    pub open_channel_auto_accept_min_ckb_funding_amount: String,
    pub auto_accept_channel_ckb_funding_amount: String,
    pub default_funding_lock_script: Script,
    pub tlc_expiry_delta: String,
    pub tlc_min_value: String,
    pub tlc_fee_proportional_millionths: String,
    pub channel_count: String,
    pub pending_channel_count: String,
    pub peers_count: String,
    #[serde(default)]
    pub udt_cfg_infos: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeerInfo {
    pub address: String,
    pub pubkey: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListPeersResult {
    pub peers: Vec<PeerInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectPeerParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewInvoiceParams {
    pub amount: String,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_preimage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_expiry_delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udt_type_script: Option<Script>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_algorithm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_mpp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_trampoline_routing: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetInvoiceParams {
    pub payment_hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettleInvoiceParams {
    pub payment_hash: String,
    pub payment_preimage: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendPaymentParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_tlc_expiry_delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlc_expiry_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_parts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trampoline_hops: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keysend: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udt_type_script: Option<Script>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_self_payment: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_records: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hop_hints: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetPaymentParams {
    pub payment_hash: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaymentResult {
    pub payment_hash: String,
    pub status: String,
    pub created_at: String,
    pub last_updated_at: String,
    pub failed_error: Option<String>,
    pub fee: String,
    pub custom_records: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvoiceData {
    pub timestamp: String,
    pub payment_hash: String,
    #[serde(default)]
    pub attrs: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CkbInvoice {
    pub currency: String,
    pub amount: String,
    pub signature: String,
    pub data: InvoiceData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvoiceResult {
    pub invoice_address: String,
    pub invoice: CkbInvoice,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenChannelParams {
    pub pubkey: String,
    pub funding_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_way: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub funding_udt_type_script: Option<Script>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown_script: Option<Script>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_delay_epoch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_fee_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub funding_fee_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlc_expiry_delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlc_min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlc_fee_proportional_millionths: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tlc_value_in_flight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tlc_number_in_flight: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenChannelResult {
    pub temporary_channel_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListChannelsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_closed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_pending: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelState {
    pub state_name: String,
    #[serde(default)]
    pub state_flags: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Channel {
    pub channel_id: String,
    pub channel_outpoint: Option<Value>,
    pub created_at: String,
    pub enabled: bool,
    pub failure_detail: Option<String>,
    pub funding_udt_type_script: Option<Value>,
    pub is_acceptor: bool,
    pub is_one_way: bool,
    pub is_public: bool,
    pub latest_commitment_transaction_hash: Option<String>,
    pub local_balance: String,
    pub offered_tlc_balance: String,
    #[serde(default)]
    pub pending_tlcs: Vec<Value>,
    pub pubkey: String,
    pub received_tlc_balance: String,
    pub remote_balance: String,
    pub shutdown_transaction_hash: Option<String>,
    pub state: ChannelState,
    pub tlc_expiry_delta: String,
    pub tlc_fee_proportional_millionths: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListChannelsResult {
    pub channels: Vec<Channel>,
}
