use super::models::*;
use super::*;
use iron::status;

/// Retrieve the current Nym network topology via HTTP
pub fn get(_req: &mut Request) -> IronResult<Response> {
    let topology = Topology {
        mix_nodes: Vec::<Mixnode>::new(),
        service_providers: Vec::<ServiceProvider>::new(),
        validators: Vec::<Validator>::new(),
    };
    let response = serde_json::to_string_pretty(&topology).unwrap();
    Ok(Response::with((status::Ok, response)))
}
