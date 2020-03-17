use serde::{Deserialize, Serialize};

// Topology shows us the current state of the overall Nym network
#[derive(Serialize, Deserialize, Debug)]
pub struct Topology {
    pub validators: Vec<Validator>,
    pub mix_nodes: Vec<MixNode>,
    pub service_providers: Vec<ServiceProvider>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Validator {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MixNode {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceProvider {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}
