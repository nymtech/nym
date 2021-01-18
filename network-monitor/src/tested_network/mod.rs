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
use crate::TIME_CHUNK_SIZE;
use gateway_client::error::GatewayClientError;
use gateway_client::GatewayClient;
use log::*;
use nymsphinx::forwarding::packet::MixPacket;
use std::time::Duration;
use topology::{gateway, mix, NymTopology};

pub(crate) mod good_topology;

pub(crate) enum TestMix {
    Valid(mix::Node, [TestPacket; 2]),
    Incompatible(mix::Node),
    Malformed(String),
}

impl TestMix {
    pub(crate) fn is_valid(&self) -> bool {
        matches!(self, TestMix::Valid(..))
    }
}

pub(crate) enum TestGateway {
    /// Indicates a presumably working gateway that is going to receive test packets
    ValidGateway(gateway::Node, [TestPacket; 2]),

    /// Indicates gateway that is not version compatible with the rest of the network
    IncompatibleGateway(gateway::Node),

    /// Indicates gateway that provided invalid information during registration and cannot
    /// be parsed into expected types
    MalformedGateway(String),
    // /// Indicates gateway that failed to accept initial connection within specified timeout interval.
    // NonRoutableGateway(gateway::Node),
}

impl TestGateway {
    pub(crate) fn is_valid(&self) -> bool {
        matches!(self, TestGateway::ValidGateway(..))
    }
}

pub(crate) struct TestedNetwork {
    system_version: String,
    // #[deprecated]
    // gateway_client: GatewayClient,
    good_v4_topology: NymTopology,
    good_v6_topology: NymTopology,
    // #[deprecated]
    // max_sending_rate: usize,
}

impl TestedNetwork {
    pub(crate) fn new_good(
        // gateway_client: GatewayClient,
        good_v4_topology: NymTopology,
        good_v6_topology: NymTopology,
        // max_sending_rate: usize,
    ) -> Self {
        TestedNetwork {
            system_version: good_v4_topology.mixes()[&1][0].version.clone(),
            // gateway_client,
            good_v4_topology,
            good_v6_topology,
            // max_sending_rate,
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

    // pub(crate) async fn start_gateway_client(&mut self) {
    //     self.gateway_client
    //         .authenticate_and_start()
    //         .await
    //         .expect("Couldn't authenticate with gateway node.");
    // }

    pub(crate) fn test_mixes(&mut self) {}

    pub(crate) fn test_gateways(&mut self) {}

    // pub(crate) async fn send_messages(
    //     &mut self,
    //     mut mix_packets: Vec<MixPacket>,
    // ) -> Result<(), GatewayClientError> {
    //     info!(target: "MessageSender", "Got {} packets to send to gateway", mix_packets.len());
    //     // if we have fewer packets than our rate, just send it all
    //     if mix_packets.len() <= self.max_sending_rate {
    //         info!(target: "MessageSender", "Everything is going to get sent as one.");
    //         self.gateway_client
    //             .batch_send_mix_packets(mix_packets)
    //             .await?;
    //     } else {
    //         let packets_per_time_chunk =
    //             (self.max_sending_rate as f64 * TIME_CHUNK_SIZE.as_secs_f64()) as usize;
    //
    //         info!(
    //             target: "MessageSender",
    //             "Going to send {} packets every {:?}",
    //             packets_per_time_chunk, TIME_CHUNK_SIZE
    //         );
    //
    //         let total_expected_time =
    //             Duration::from_secs_f64(mix_packets.len() as f64 / self.max_sending_rate as f64);
    //         info!(target: "MessageSender",
    //               "With our rate of {} packets/s it should take around {:?} to send it all...",
    //               self.max_sending_rate, total_expected_time
    //         );
    //
    //         // TODO: is it perhaps possible to avoid so many reallocations here?
    //         loop {
    //             let mut retained = mix_packets.split_off(packets_per_time_chunk);
    //
    //             let is_last = retained.len() < packets_per_time_chunk;
    //
    //             debug!(target: "MessageSender", "Sending {} packets...", mix_packets.len());
    //             if mix_packets.len() == 1 {
    //                 self.gateway_client
    //                     .send_mix_packet(mix_packets.pop().unwrap())
    //                     .await?;
    //             } else {
    //                 self.gateway_client
    //                     .batch_send_mix_packets(mix_packets)
    //                     .await?;
    //             }
    //
    //             tokio::time::delay_for(TIME_CHUNK_SIZE).await;
    //
    //             if is_last {
    //                 debug!(target: "MessageSender", "Sending {} packets...", retained.len());
    //                 if retained.len() == 1 {
    //                     self.gateway_client
    //                         .send_mix_packet(retained.pop().unwrap())
    //                         .await?;
    //                 } else {
    //                     self.gateway_client.batch_send_mix_packets(retained).await?;
    //                 }
    //                 break;
    //             }
    //             mix_packets = retained
    //         }
    //         info!(target: "MessageSender", "Done sending");
    //     }
    //
    //     Ok(())
    // }

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
}
