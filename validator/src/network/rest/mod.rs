use crate::services::mixmining;
use iron::prelude::*;
use presence::announcements;
use router::Router;

mod presence;

pub struct Api {
    mixmining_service: mixmining::Service,
}

impl Api {
    pub fn new(mixmining_service: mixmining::Service) -> Api {
        Api { mixmining_service }
    }

    pub async fn run(self) {
        let port = 3000;
        println!("* starting REST API on localhost:{}", port);

        let mixnode_announcement = announcements::MixnodeHandler::new(self.mixmining_service);

        let mut router = Router::new();
        router.get("/topology", presence::topology::get, "topology_get");
        router.post(
            "/presence/announcements",
            mixnode_announcement,
            "presence_announcements_post",
        );

        Iron::new(router)
            .http(format!("localhost:{}", port))
            .unwrap();
    }
}
