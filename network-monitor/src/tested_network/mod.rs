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

use crate::test_packet::{IpVersion, TestPacket};
use gateway_client::error::GatewayClientError;
use gateway_client::GatewayClient;
use nymsphinx::forwarding::packet::MixPacket;
use topology::{gateway, mix, NymTopology};

mod good_topology;

pub(crate) enum TestMix {
    ValidMix(mix::Node, [TestPacket; 2]),
    IncompatibleMix(mix::Node),
    MalformedMix(String),
}

impl TestMix {
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            TestMix::ValidMix(..) => true,
            _ => false,
        }
    }
}

pub(crate) struct TestedNetwork {
    system_version: String,
    gateway_client: GatewayClient,
    good_v4_topology: NymTopology,
    good_v6_topology: NymTopology,
}

pub(crate) fn v4_gateway() -> gateway::Node {
    good_topology::v4_gateway()
}

impl TestedNetwork {
    pub(crate) fn new_good(gateway_client: GatewayClient) -> Self {
        let good_v4_topology = good_topology::new_v4();

        TestedNetwork {
            system_version: good_v4_topology.mixes()[&1][0].version.clone(),
            gateway_client,
            good_v4_topology,
            good_v6_topology: good_topology::new_v6(),
        }
    }

    pub(crate) fn system_version(&self) -> &str {
        &self.system_version
    }

    pub(crate) async fn start_gateway_client(&mut self) {
        self.gateway_client
            .authenticate_and_start()
            .await
            .expect("Couldn't authenticate with gateway node.");
    }

    pub(crate) async fn send_messages(
        &mut self,
        mix_packets: Vec<MixPacket>,
    ) -> Result<(), GatewayClientError> {
        self.gateway_client
            .batch_send_mix_packets(mix_packets)
            .await?;
        Ok(())
    }

    pub(crate) fn substitute_node(&self, node: mix::Node, ip_version: IpVersion) -> NymTopology {
        let mut good_topology = match ip_version {
            IpVersion::V4 => self.good_v4_topology.clone(),
            IpVersion::V6 => self.good_v6_topology.clone(),
        };

        good_topology.set_mixes_in_layer(node.layer as u8, vec![node]);
        good_topology
    }
}
