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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayPresence {
    pub location: String,
    pub client_listener: String,
    pub mixnet_listener: String,
    pub identity_key: String,
    pub sphinx_key: String,
    pub registered_clients: Vec<GatewayClient>,
    pub last_seen: u64,
    pub version: String,
}

impl Into<topology::gateway::Node> for GatewayPresence {
    fn into(self) -> topology::gateway::Node {
        topology::gateway::Node {
            location: self.location,
            client_listener: self.client_listener.parse().unwrap(),
            mixnet_listener: self.mixnet_listener.parse().unwrap(),
            identity_key: self.identity_key,
            sphinx_key: self.sphinx_key,
            registered_clients: self
                .registered_clients
                .into_iter()
                .map(|c| c.into())
                .collect(),
            last_seen: self.last_seen,
            version: self.version,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayClient {
    pub pub_key: String,
}

impl Into<topology::gateway::Client> for GatewayClient {
    fn into(self) -> topology::gateway::Client {
        topology::gateway::Client {
            pub_key: self.pub_key,
        }
    }
}
