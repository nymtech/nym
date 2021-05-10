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

use crate::rtt_measurement::listener::PacketListener;
use crate::rtt_measurement::sender::{PacketSender, TestedNode};
use crypto::asymmetric::identity;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use itertools::Itertools;
use log::*;
pub(crate) use node_result::NodeResult;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

pub mod error;
pub(crate) mod listener;
pub(crate) mod node_result;
pub(crate) mod packet;
pub(crate) mod sender;

pub(crate) type MeasurementResults = HashMap<[u8; identity::PUBLIC_KEY_LENGTH], Option<NodeResult>>;

pub struct RttMeasurer {
    packet_sender: Arc<PacketSender>,
    packet_listener: Arc<PacketListener>,

    batch_size: usize,
    testing_interval: Duration,
}

impl RttMeasurer {
    // TODO: configs etc
    pub fn new(listening_address: SocketAddr, identity: Arc<identity::KeyPair>) -> Self {
        // tmp:
        let packets_per_node = 10;
        let packet_timeout = Duration::from_secs(2);
        let delay_between_packets = Duration::from_millis(200);
        let batch_size = 10;

        RttMeasurer {
            packet_sender: Arc::new(PacketSender::new(
                Arc::clone(&identity),
                packets_per_node,
                packet_timeout,
                delay_between_packets,
            )),
            packet_listener: Arc::new(PacketListener::new(
                listening_address,
                Arc::clone(&identity),
            )),
            batch_size,
            testing_interval: Default::default(),
        }
    }

    fn start_listening(&self) -> JoinHandle<()> {
        let packet_listener = Arc::clone(&self.packet_listener);
        tokio::spawn(packet_listener.run())
    }

    async fn perform_measurement(&self, nodes_to_test: &[TestedNode]) -> MeasurementResults {
        let mut results = HashMap::with_capacity(nodes_to_test.len());

        for chunk in &nodes_to_test.iter().chunks(self.batch_size) {
            let mut measurement_chunk = chunk
                .into_iter()
                .map(|node| {
                    let node = *node;
                    let packet_sender = Arc::clone(&self.packet_sender);
                    // TODO: there's a potential issue here. if we make the measurement go into separate
                    // task, we risk biasing results with the bunch of context switches overhead
                    // but if we don't do it, it will take ages to complete

                    // TODO: check performance difference when it's not spawned as a separate task
                    tokio::spawn(async move {
                        (
                            packet_sender.send_packets_to_node(node).await,
                            node.identity,
                        )
                    })
                })
                .collect::<FuturesUnordered<_>>();

            // exhaust the results
            while let Some(result) = measurement_chunk.next().await {
                // if we receive JoinError it means the task failed to get executed, so either there's a bigger issue with tokio
                // or there was a panic inside the task itself. In either case, we should just terminate ourselves.
                let execution_result = result.expect("the measurement task panicked!");
                let measurement_result = match execution_result.0 {
                    Err(err) => {
                        warn!(
                            "Failed to perform measurement for {} - {}",
                            execution_result.1.to_base58_string(),
                            err
                        );
                        None
                    }
                    Ok(result) => Some(result),
                };
                results.insert(execution_result.1.to_bytes(), measurement_result);
            }
        }

        results
    }

    pub async fn run(&self) {

        // let listener_jh = self.start_listening();
        //
        // listener_jh.await;

        //
    }
}
