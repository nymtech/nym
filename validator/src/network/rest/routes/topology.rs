use super::*;
use crate::network::rest::models::presence;

pub fn get(req: &mut Request) -> IronResult<Response> {
    let topology = presence::Topology {
        mix_nodes: Vec::<presence::MixNode>::new(),
        service_providers: Vec::<presence::ServiceProvider>::new(),
        validators: Vec::<presence::Validator>::new(),
    };
    let resp = serde_json::to_string_pretty(&topology).unwrap();
    Ok(Response::with((status::Ok, resp)))
}
