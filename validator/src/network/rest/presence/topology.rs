use super::*;
use iron::status;
use serde::{Deserialize, Serialize};

/// Retrieve the current Nym network topology via HTTP
pub fn get(_req: &mut Request) -> IronResult<Response> {
    let topology = Topology {
        mix_nodes: Vec::<MixNode>::new(),
        service_providers: Vec::<ServiceProvider>::new(),
        validators: Vec::<Validator>::new(),
    };
    let response = serde_json::to_string_pretty(&topology).unwrap();
    Ok(Response::with((status::Ok, response)))
}

/// Topology shows us the current state of the overall Nym network
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
