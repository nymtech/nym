use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedMixMetric {
    pub pub_key: String,
    pub received: u64,
    pub sent: HashMap<String, u64>,
    pub timestamp: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixMetric {
    pub pub_key: String,
    pub received: u64,
    pub sent: HashMap<String, u64>,
}
