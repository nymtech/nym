use crate::services::mixmining;
use iron::prelude::*;
use presence::mixnode;
use presence::topology;
use router::Router;
use std::sync::{Arc, Mutex};

mod presence;

pub struct Api {
    mixmining_service: Arc<Mutex<mixmining::Service>>,
}

impl Api {
    pub fn new(mixmining_service: mixmining::Service) -> Api {
        let service = Arc::new(Mutex::new(mixmining_service));
        Api {
            mixmining_service: service,
        }
    }

    /// Run the REST API.
    pub async fn run(self) {
        let port = 3000; // TODO: make this configurable
        let address = format!("localhost:{}", port);
        println!("* starting REST API on http://{}", address);

        let router = self.setup_router();

        Iron::new(router).http(address).unwrap();
    }

    /// Tie together URL route paths with handler functions.
    fn setup_router(self) -> Router {
        // define a Router to hold our routes
        let mut router = Router::new();

        // set up handlers
        let create_mixnode_presence =
            mixnode::CreatePresence::new(Arc::clone(&self.mixmining_service));
        let get_topology = topology::GetTopology::new(Arc::clone(&self.mixmining_service));

        // tie routes to handlers
        router.get("/topology", get_topology, "topology_get");
        router.post(
            "/presence/mixnodes",
            create_mixnode_presence,
            "presence_mixnodes_post",
        );

        router
    }
}
