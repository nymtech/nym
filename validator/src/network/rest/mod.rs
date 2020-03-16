use iron::prelude::*;
use router::Router;

mod models;
mod routes;

pub struct Api {}

impl Api {
    pub fn new() -> Api {
        Api {}
    }

    pub async fn run(self) {
        let port = 3000;
        println!("* starting REST API on localhost:{}", port);

        let router = self.setup_routes();
        Iron::new(router)
            .http(format!("localhost:{}", port))
            .unwrap();
    }

    pub fn setup_routes(&self) -> Router {
        let mut router = Router::new();
        router.get("/topology", routes::topology::get, "topology_get");
        router
    }
}
