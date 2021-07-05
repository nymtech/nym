// Copyright 2021 Nym Technologies SA
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

use crate::network_monitor::test_packet::IpVersion;
use topology::{gateway, mix, NymTopology};

pub(crate) mod good_topology;

#[derive(Clone)]
pub(crate) struct TestedNetwork {
    system_version: String,
    good_v4_topology: NymTopology,
    good_v6_topology: NymTopology,
}

impl TestedNetwork {
    pub(crate) fn new_good(good_v4_topology: NymTopology, good_v6_topology: NymTopology) -> Self {
        TestedNetwork {
            system_version: "0.10.0".to_string(),
            good_v4_topology,
            good_v6_topology,
        }
    }

    pub(crate) fn main_v4_gateway(&self) -> &gateway::Node {
        if self.good_v4_topology.gateways().len() > 1 {
            warn!("we have more than a single 'good' gateway and in few places we made assumptions that only a single one existed!")
        }

        self.good_v4_topology
            .gateways()
            .get(0)
            .expect("our good v4 topology does not have any gateway specified!")
    }

    pub(crate) fn system_version(&self) -> &str {
        &self.system_version
    }

    pub(crate) fn substitute_mix(&self, node: mix::Node, ip_version: IpVersion) -> NymTopology {
        let mut good_topology = match ip_version {
            IpVersion::V4 => self.good_v4_topology.clone(),
            IpVersion::V6 => self.good_v6_topology.clone(),
        };

        good_topology.set_mixes_in_layer(node.layer as u8, vec![node]);
        good_topology
    }

    pub(crate) fn substitute_gateway(
        &self,
        gateway: gateway::Node,
        ip_version: IpVersion,
    ) -> NymTopology {
        let mut good_topology = match ip_version {
            IpVersion::V4 => self.good_v4_topology.clone(),
            IpVersion::V6 => self.good_v6_topology.clone(),
        };

        good_topology.set_gateways(vec![gateway]);
        good_topology
    }

    pub(crate) fn v4_topology(&self) -> &NymTopology {
        &self.good_v4_topology
    }

    pub(crate) fn v6_topology(&self) -> &NymTopology {
        &self.good_v6_topology
    }
}
