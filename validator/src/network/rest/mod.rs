use iron::prelude::*;
use presence::announcements;
use router::Router;

mod presence;

pub struct Api {}

impl Api {
    pub fn new() -> Api {
        Api {}
    }

    pub async fn run(self) {
        let port = 3000;
        println!("* starting REST API on localhost:{}", port);

        let mut router = Router::new();
        router.get("/topology", presence::topology::get, "topology_get");
        router.post(
            "/presence/announcements",
            announcements::post,
            "presence_announcements_post",
        );

        Iron::new(router)
            .http(format!("localhost:{}", port))
            .unwrap();
    }
}
