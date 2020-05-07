use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mixnode {
    pub host: String,
    pub public_key: String,
    pub version: String,
    pub location: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProvider {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}

/// Topology shows us the current state of the overall Nym network
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub mixnodes: Vec<Mixnode>,
    pub service_providers: Vec<ServiceProvider>,
    pub validators: Vec<Validator>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Validator {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct Timestamp(u64);

impl Default for Timestamp {
    fn default() -> Timestamp {
        Timestamp(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        )
    }
}

impl From<u64> for Timestamp {
    fn from(v: u64) -> Timestamp {
        Timestamp(v)
    }
}

impl Into<u64> for Timestamp {
    fn into(self) -> u64 {
        self.0
    }
}
