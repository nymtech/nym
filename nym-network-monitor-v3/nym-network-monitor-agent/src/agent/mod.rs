// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::Config;
use crate::agent::result::{PacketsStatistics, TestRunResult};
use crate::agent::tested_node::TestedNodeDetails;
use crate::egress_connection::EgressConnection;
use crate::listener::MixnetListener;
use crate::listener::received::MixnetPacketsSender;
use crate::processor::{MixnetPacketProcessor, PayloadRecovery, ProcessedPacket};
use crate::sphinx_helpers::{build_test_sphinx_packet, create_test_sphinx_packet_header};
use crate::test_packet::{TestPacketContent, TestPacketHeader};
use humantime::format_duration;
use nym_crypto::asymmetric::x25519;
use nym_noise::config::{NoiseConfig, NoiseNetworkView};
use nym_pemstore::load_key;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_types::SphinxPacket;
use nym_task::ShutdownToken;
use rand::rngs::OsRng;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::pin;
use tokio::time::{Instant, sleep};
use tracing::{debug, error, info, warn};

mod config;
mod result;
mod tested_node;

pub(crate) struct NetworkMonitorAgent {
    config: Config,

    packet_counter: u64,

    reusable_test_header: Option<TestPacketHeader>,

    noise_key: Arc<x25519::KeyPair>,

    sphinx_key: Arc<x25519::KeyPair>,

    tested_node: TestedNodeDetails,
}

fn as_sphinx_node(
    address: SocketAddr,
    pub_key: x25519::PublicKey,
) -> anyhow::Result<nym_sphinx_types::Node> {
    Ok(nym_sphinx_types::Node::new(
        NymNodeRoutingAddress::from(address).try_into()?,
        pub_key.into(),
    ))
}

impl NetworkMonitorAgent {
    fn new<P: AsRef<Path>>(
        config: Config,
        noise_key_path: P,
        tested_node: TestedNodeDetails,
    ) -> anyhow::Result<Self> {
        let noise_key: x25519::PrivateKey = load_key(noise_key_path)?;
        let sphinx_key = x25519::PrivateKey::new(&mut OsRng);

        let reusable_test_header = if config.reuse_header {
            // we don't want any delays
            // and the packet route is test node -> this client
            let route = vec![
                tested_node.as_sphinx_node()?,
                as_sphinx_node(config.mixnet_address, sphinx_key.public_key())?,
            ];
            let delay = config.packet_delay;
            Some(create_test_sphinx_packet_header(route, delay)?)
        } else {
            None
        };

        todo!()
        // Self {
        //     config,
        //     reusable_test_header,
        //     noise_key,
        //     sphinx_key,
        //     tested_node,
        // }
    }
    async fn establish_egress_connection(&self) -> anyhow::Result<EgressConnection> {
        EgressConnection::establish(
            self.config.mixnet_address,
            self.config.egress_connection_timeout,
            self.tested_node.key_rotation,
            &self.noise_config(),
        )
        .await
    }

    fn build_packet_processor(&self) -> MixnetPacketProcessor {
        let packet_recovery = match &self.reusable_test_header {
            Some(header) => header.clone().into(),
            None => self.sphinx_key.clone().into(),
        };
        MixnetPacketProcessor::new(packet_recovery, self.config.waiting_duration)
    }

    async fn build_mixnet_listener(
        &self,
        received_sender: MixnetPacketsSender,
        shutdown_token: ShutdownToken,
    ) -> anyhow::Result<MixnetListener> {
        MixnetListener::new(
            self.config.mixnet_address,
            self.tested_node.address,
            self.noise_config(),
            received_sender,
            shutdown_token.clone(),
        )
        .await
    }

    fn noise_config(&self) -> NoiseConfig {
        let mut nodes = HashMap::new();
        nodes.insert(
            self.tested_node.address.ip(),
            self.tested_node.as_noise_node(),
        );
        let network = NoiseNetworkView::new(nodes);

        NoiseConfig::new(
            self.noise_key.clone(),
            network,
            self.config.noise_handshake_timeout,
        )
    }

    fn as_sphinx_node(&self) -> anyhow::Result<nym_sphinx_types::Node> {
        as_sphinx_node(self.config.mixnet_address, *self.sphinx_key.public_key())
    }

    fn create_test_sphinx_packet(&mut self) -> anyhow::Result<SphinxPacket> {
        let content = TestPacketContent::new(self.packet_counter);
        self.packet_counter += 1;

        match &self.reusable_test_header {
            Some(header) => header.create_test_packet(content),
            None => {
                let route = vec![self.tested_node.as_sphinx_node()?, self.as_sphinx_node()?];
                build_test_sphinx_packet(
                    &route,
                    self.config.packet_delay,
                    None,
                    &content.to_bytes(),
                )
            }
        }
    }

    fn create_packet_batch(&mut self, batch_size: usize) -> anyhow::Result<Vec<SphinxPacket>> {
        let mut packets = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let packet = self.create_test_sphinx_packet()?;
            packets.push(packet);
        }
        Ok(packets)
    }

    fn packet_latency(&self, received: ProcessedPacket) -> Duration {
        // make sure to subtract the sphinx delay from the RTT
        received.rtt - self.config.packet_delay
    }

    // only return error on critical, agent, failure. not on tested node issues
    pub(crate) async fn run_stress_test(&mut self) -> anyhow::Result<TestRunResult> {
        let mut test_result = TestRunResult::new_empty();

        // 1. establish the connection - if it fails, there's no point in continuing
        let mut egress_connection = match self.establish_egress_connection().await {
            Ok(conn) => conn,
            Err(err) => {
                test_result.set_error(
                    err.context("failed to establish egress node connection")
                        .to_string(),
                );
                return Ok(test_result);
            }
        };

        let mut processor = self.build_packet_processor();
        let shutdown_token = ShutdownToken::new();
        let mut listener = self
            .build_mixnet_listener(processor.sender(), shutdown_token.clone())
            .await?;

        // 2. spawn the mixnet packet listener that forwards any received packets to the processor
        let listener_join = tokio::spawn(async move { listener.run().await });

        // 3. send a single packet to see if the node is even going to respond to it
        let test_packet = self.create_test_sphinx_packet()?;

        info!("sending initial packet");
        egress_connection.send_packet(test_packet).await?;
        match processor.next_packet().await {
            Ok(res) => {
                info!("received {res}");
                let latency = self.packet_latency(res);
                test_result.set_approximate_latency(latency);
            }
            Err(err) => {
                test_result.set_error(
                    err.context("failed to receive a valid initial packet back")
                        .to_string(),
                );
                return Ok(test_result);
            }
        }

        // 4. send it again to check if the node is configured correctly for testing
        // (i.e. whether the agent can bypass the bloomfilter)
        if self.config.reuse_header {
            info!("repeating the packet to check bloomfilter bypass configuration");
            let test_packet = self.create_test_sphinx_packet()?;
            egress_connection.send_packet(test_packet).await?;

            match processor.next_packet().await {
                Ok(res) => {
                    info!("received {res}")
                }
                Err(err) => {
                    test_result.set_error(
                        err.context("failed to receive a valid secondary packet back - the node might not have a working chain subscriber (or the agent might be misconfigured)")
                            .to_string(),
                    );
                    return Ok(test_result);
                }
            }
        }

        // 5. finally, send the packets at the pre-defined rate to see if it can handle the target load
        info!(
            "beginning the proper load testing. going to send send at rate {}/s for {}",
            self.config.target_rate,
            format_duration(self.config.sending_duration)
        );

        // one batch every (sending_batch_size / target_rate) seconds keeps us at the target rate
        let batch_interval = self.config.batch_interval();
        let mut interval = tokio::time::interval(batch_interval);
        // if we fall behind, don't try to catch up with burst sends
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let start = Instant::now();
        let mut sent = 0;
        let total_packets = self.config.expected_packets();

        loop {
            if start.elapsed() >= self.config.sending_duration {
                break;
            }
            if sent >= total_packets {
                break;
            }
            interval.tick().await;

            // the last batch may be smaller than other batches
            let remaining = total_packets - sent;
            let batch_size = self.config.sending_batch_size.min(remaining);
            let batch = self.create_packet_batch(batch_size)?;

            egress_connection.send_packet_batch(batch).await?;
            sent += batch_size;
        }

        if sent < total_packets {
            warn!(
                "did not manage to send all required packets within the sending window. sent {sent}/{total_packets}"
            );
        }

        // empty the entire channel and then wait for at most configured amount of time
        let mut received = processor.all_available();
        if received.len() < sent {
            let deadline = sleep(self.config.waiting_duration);
            pin!(deadline);
            loop {
                tokio::select! {
                    _ = &mut deadline => {
                        break;
                    }
                    next = processor.next_packet() => {
                        received.push(next)
                    }
                }
            }
        }

        // process received
        let mut valid_received = HashMap::new();
        for packet in received {
            let Ok(packet) = packet else {
                debug!("received packet was malformed");
                continue;
            };
            if valid_received.insert(packet.id, packet).is_some() {
                error!(
                    "‼️ received duplicate packet for id {} - something nasty is going on!",
                    packet.id
                );
                test_result.set_received_duplicates();
            }
        }

        let latencies = valid_received
            .values()
            .map(|p| self.packet_latency(*p))
            .collect::<Vec<_>>();

        let stats = PacketsStatistics::compute(&latencies);

        test_result.set_packets_statistics(stats);
        test_result.set_packets_sent(sent);
        test_result.set_packets_received(valid_received.len());

        Ok(test_result)
    }
}
