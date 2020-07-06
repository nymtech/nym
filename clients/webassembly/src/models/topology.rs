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

use serde::Serializer;
use std::convert::TryInto;
use topology::NymTopology;

#[derive(Clone, Debug)]
pub struct Topology {
    inner: directory_client_models::presence::Topology,
}

impl serde::Serialize for Topology {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl Topology {
    pub fn new(json: &str) -> Self {
        if json.is_empty() {
            panic!("empty json passed");
        }

        Topology {
            inner: serde_json::from_str(json).unwrap(),
        }
    }

    #[cfg(test)]
    pub(crate) fn set_mixnodes(
        &mut self,
        mix_nodes: Vec<directory_client_models::presence::mixnodes::MixNodePresence>,
    ) {
        self.inner.mix_nodes = mix_nodes
    }

    #[cfg(test)]
    pub(crate) fn get_current_raw_mixnodes(
        &self,
    ) -> Vec<directory_client_models::presence::mixnodes::MixNodePresence> {
        self.inner.mix_nodes.clone()
    }
}

impl TryInto<NymTopology> for Topology {
    type Error = directory_client_models::presence::TopologyConversionError;

    fn try_into(self) -> Result<NymTopology, Self::Error> {
        self.inner.try_into()
    }
}
