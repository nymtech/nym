use serde::{Deserialize, Serialize};

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
