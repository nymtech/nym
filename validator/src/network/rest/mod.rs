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
