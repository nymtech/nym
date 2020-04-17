use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mixnode {
    pub host: String,
    pub public_key: String,
    pub last_seen: u64,
    pub location: String,
    pub stake: u64,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceProvider {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}

/// Topology shows us the current state of the overall Nym network
#[derive(Serialize, Deserialize, Debug)]
pub struct Topology {
    pub mixnodes: Vec<Mixnode>,
    pub service_providers: Vec<ServiceProvider>,
    pub validators: Vec<Validator>,
}

impl Topology {
    pub fn new(
        mixnodes: Vec<Mixnode>,
        service_providers: Vec<ServiceProvider>,
        validators: Vec<Validator>,
    ) -> Topology {
        Topology {
            mixnodes,
            service_providers,
            validators,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Validator {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}
