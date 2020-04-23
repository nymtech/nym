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
use topology::provider;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProviderPresence {
    pub location: String,
    pub client_listener: String,
    pub mixnet_listener: String,
    pub pub_key: String,
    pub registered_clients: Vec<MixProviderClient>,
    pub last_seen: u64,
    pub version: String,
}

impl Into<topology::provider::Node> for MixProviderPresence {
    fn into(self) -> topology::provider::Node {
        topology::provider::Node {
            location: self.location,
            client_listener: self.client_listener.parse().unwrap(),
            mixnet_listener: self.mixnet_listener.parse().unwrap(),
            pub_key: self.pub_key,
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

impl From<topology::provider::Node> for MixProviderPresence {
    fn from(mpn: provider::Node) -> Self {
        MixProviderPresence {
            location: mpn.location,
            client_listener: mpn.client_listener.to_string(),
            mixnet_listener: mpn.mixnet_listener.to_string(),
            pub_key: mpn.pub_key,
            registered_clients: mpn
                .registered_clients
                .into_iter()
                .map(|c| c.into())
                .collect(),
            last_seen: mpn.last_seen,
            version: mpn.version,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProviderClient {
    pub pub_key: String,
}

impl Into<topology::provider::Client> for MixProviderClient {
    fn into(self) -> topology::provider::Client {
        topology::provider::Client {
            pub_key: self.pub_key,
        }
    }
}

impl From<topology::provider::Client> for MixProviderClient {
    fn from(mpc: topology::provider::Client) -> Self {
        MixProviderClient {
            pub_key: mpc.pub_key,
        }
    }
}
