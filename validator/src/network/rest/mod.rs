// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use crate::services::mixmining;
use iron::prelude::*;
use presence::mixnode;
use presence::topology;
use router::Router;
use std::sync::{Arc, Mutex};

mod capacity;
mod presence;
mod staking;

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
        let capacity_update = capacity::Update::new(Arc::clone(&self.mixmining_service));
        let capacity_get = capacity::Get::new(Arc::clone(&self.mixmining_service));
        let presence_mixnode_create =
            mixnode::CreatePresence::new(Arc::clone(&self.mixmining_service));
        let topology_get = topology::GetTopology::new(Arc::clone(&self.mixmining_service));

        // tie routes to handlers
        router.get("/capacity", capacity_get, "capacity_get");
        router.post("/capacity", capacity_update, "capacity_update");
        router.get("/topology", topology_get, "topology_get");
        router.post(
            "/presence/mixnodes",
            presence_mixnode_create,
            "presence_mixnodes_post",
        );

        router
    }
}
