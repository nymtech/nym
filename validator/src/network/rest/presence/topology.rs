use super::*;
use iron::status;
use iron::Handler;

pub struct GetTopology {
    service: Arc<Mutex<mixmining::Service>>,
}

impl GetTopology {
    pub fn new(service: Arc<Mutex<mixmining::Service>>) -> GetTopology {
        GetTopology { service }
    }
}

impl Handler for GetTopology {
    fn handle(&self, _req: &mut Request) -> IronResult<Response> {
        println!("Getting topology!...");
        let service_topology = self.service.lock().unwrap().topology();
        let topology = service_topology; //models::Topology::from(service_topology);
        let response = serde_json::to_string_pretty(&topology).unwrap();
        Ok(Response::with((status::Ok, response)))
    }
}
