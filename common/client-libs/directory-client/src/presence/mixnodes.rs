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
use std::convert::TryInto;
use std::io;
use std::net::ToSocketAddrs;
use topology::mix;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixNodePresence {
    pub location: String,
    pub host: String,
    pub pub_key: String,
    pub layer: u64,
    pub last_seen: u64,
    pub version: String,
}

impl TryInto<topology::mix::Node> for MixNodePresence {
    type Error = io::Error;

    fn try_into(self) -> Result<topology::mix::Node, Self::Error> {
        let resolved_hostname = self.host.to_socket_addrs()?.next();
        if resolved_hostname.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "no valid socket address",
            ));
        }

        Ok(topology::mix::Node {
            location: self.location,
            host: resolved_hostname.unwrap(),
            pub_key: self.pub_key,
            layer: self.layer,
            last_seen: self.last_seen,
            version: self.version,
        })
    }
}

impl From<topology::mix::Node> for MixNodePresence {
    fn from(mn: mix::Node) -> Self {
        MixNodePresence {
            location: mn.location,
            host: mn.host.to_string(),
            pub_key: mn.pub_key,
            layer: mn.layer,
            last_seen: mn.last_seen,
            version: mn.version,
        }
    }
}
