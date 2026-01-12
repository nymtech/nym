use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct BridgeInformation {
    pub version: String,
    pub transports: Vec<BridgeParameters>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transport_type", content = "args")]
#[serde(rename_all = "snake_case")]
pub enum BridgeParameters {
    QuicPlain(QuicClientOptions),
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct QuicClientOptions {
    pub addresses: Vec<String>,
    pub host: Option<String>,
    pub id_pubkey: String,
}
