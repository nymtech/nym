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
