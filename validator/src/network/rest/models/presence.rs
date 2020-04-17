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

use serde::{Deserialize, Serialize};

// Topology shows us the current state of the overall Nym network
#[derive(Serialize, Deserialize, Debug)]
pub struct Topology {
    pub validators: Vec<Validator>,
    pub mix_nodes: Vec<MixNode>,
    pub service_providers: Vec<ServiceProvider>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Validator {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MixNode {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceProvider {
    host: String,
    public_key: String,
    version: String,
    last_seen: u64,
    location: String,
}
