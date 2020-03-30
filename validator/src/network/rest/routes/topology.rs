use super::*;
use crate::network::rest::models::presence::{MixNode, ServiceProvider, Topology, Validator};

pub fn get(_req: &mut Request) -> IronResult<Response> {
    let topology = Topology {
        mix_nodes: Vec::<MixNode>::new(),
        service_providers: Vec::<ServiceProvider>::new(),
        validators: Vec::<Validator>::new(),
    };
    let response = serde_json::to_string_pretty(&topology).unwrap();
    Ok(Response::with((status::Ok, response)))
}
