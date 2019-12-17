use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CocoPresence {
    pub host: String,
    pub pub_key: String,
    pub last_seen: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixNodePresence {
    pub host: String,
    pub pub_key: String,
    pub layer: u64,
    pub last_seen: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProviderPresence {
    pub host: String,
    pub pub_key: String,
    pub registered_clients: Vec<MixProviderClient>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProviderClient {
    pub pub_key: String,
}

// Topology shows us the current state of the overall Nym network
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub coco_nodes: Vec<CocoPresence>,
    pub mix_nodes: Vec<MixNodePresence>,
    pub mix_provider_nodes: Vec<MixProviderPresence>,
}
