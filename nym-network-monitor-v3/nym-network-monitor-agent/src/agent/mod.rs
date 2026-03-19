// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::Config;
use crate::agent::result::{LatencyDistribution, TestRunResult};
use crate::agent::tested_node::TestedNodeDetails;
use crate::egress_connection::EgressConnection;
use crate::listener::MixnetListener;
use crate::listener::received::MixnetPacketsSender;
use crate::processor::{MixnetPacketProcessor, ProcessedPacket};
use crate::sphinx_helpers::{build_test_sphinx_packet, create_test_sphinx_packet_header};
use crate::test_packet::{TestPacketContent, TestPacketHeader};
use anyhow::Context;
use humantime::format_duration;
use nym_crypto::asymmetric::x25519;
use nym_noise::config::{NoiseConfig, NoiseNetworkView};
use nym_pemstore::load_key;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_types::SphinxPacket;
use nym_task::ShutdownToken;
use rand::rngs::OsRng;
use std::collections::HashMap;
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

fn as_sphinx_node(address: SocketAddr, pub_key: x25519::PublicKey) -> nym_sphinx_types::Node {
    // SAFETY: we know that the address is valid, so we can safely unwrap it
    #[allow(clippy::unwrap_used)]
    nym_sphinx_types::Node::new(
        NymNodeRoutingAddress::from(address).try_into().unwrap(),
        pub_key.into(),
    )
}

impl NetworkMonitorAgent {
    pub(crate) fn new<P: AsRef<Path>>(
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
                tested_node.as_sphinx_node(),
                as_sphinx_node(config.mixnet_address, sphinx_key.public_key()),
            ];
            let delay = config.packet_delay;
            Some(create_test_sphinx_packet_header(route, delay)?)
        } else {
            None
        };

        Ok(Self {
            config,
            packet_counter: 0,
            reusable_test_header,
            noise_key: Arc::new(noise_key.into()),
            sphinx_key: Arc::new(sphinx_key.into()),
            tested_node,
        })
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

    fn as_sphinx_node(&self) -> nym_sphinx_types::Node {
        as_sphinx_node(self.config.mixnet_address, *self.sphinx_key.public_key())
    }

    fn create_test_sphinx_packet(&mut self) -> anyhow::Result<SphinxPacket> {
        let content = TestPacketContent::new(self.packet_counter);
        self.packet_counter += 1;

        match &self.reusable_test_header {
            Some(header) => header.create_test_packet(content),
            None => {
                let route = vec![self.tested_node.as_sphinx_node(), self.as_sphinx_node()];
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

    async fn send_test_packet(
        &mut self,
        egress: &mut EgressConnection,
        result: &mut TestRunResult,
    ) -> anyhow::Result<bool> {
        let packet = self
            .create_test_sphinx_packet()
            .context("sphinx packet creation failure!")?;
        if let Err(err) = egress.send_packet(packet).await {
            result.set_error(err.context("failed to send test packet").to_string());
            return Ok(false);
        };
        Ok(true)
    }

    async fn send_test_packet_batch(
        &mut self,
        batch_size: usize,
        egress: &mut EgressConnection,
        result: &mut TestRunResult,
    ) -> anyhow::Result<bool> {
        let batch = self
            .create_packet_batch(batch_size)
            .context("sphinx packet batch creation failure!")?;

        if let Err(err) = egress.send_packet_batch(batch).await {
            result.set_error(err.context("failed to send test packet").to_string());
            return Ok(false);
        };
        Ok(true)
    }

    /// Sends a single packet and waits for it to come back.
    /// On success, sets `approximate_latency` on the result and returns `true`.
    /// On failure, sets an error on the result and returns `false` (caller should abort).
    async fn send_connectivity_probe(
        &mut self,
        egress: &mut EgressConnection,
        processor: &mut MixnetPacketProcessor,
        result: &mut TestRunResult,
    ) -> anyhow::Result<bool> {
        info!("sending initial packet");
        if !self.send_test_packet(egress, result).await? {
            return Ok(false);
        }

        match processor.next_packet().await {
            Ok(res) => {
                info!("received {res}");
                result.set_approximate_latency(self.packet_latency(res));
                Ok(true)
            }
            Err(err) => {
                result.set_error(
                    err.context("failed to receive a valid initial packet back")
                        .to_string(),
                );
                Ok(false)
            }
        }
    }

    /// Replays a packet to verify that the node's bloomfilter bypass is correctly configured.
    /// Returns `true` if the packet was returned, `false` if the node failed the check (caller should abort).
    /// Should only be called when `config.reuse_header` is set.
    async fn send_bloomfilter_probe(
        &mut self,
        egress: &mut EgressConnection,
        processor: &mut MixnetPacketProcessor,
        result: &mut TestRunResult,
    ) -> anyhow::Result<bool> {
        info!("repeating the packet to check bloomfilter bypass configuration");
        if !self.send_test_packet(egress, result).await? {
            return Ok(false);
        }

        match processor.next_packet().await {
            Ok(res) => {
                info!("received {res}");
                Ok(true)
            }
            Err(err) => {
                result.set_error(
                    err.context("failed to receive a valid secondary packet back - the node might not have a working chain subscriber (or the agent might be misconfigured)")
                        .to_string(),
                );
                Ok(false)
            }
        }
    }

    /// Sends packets at the configured rate for the configured duration.
    async fn send_load_test(
        &mut self,
        egress: &mut EgressConnection,
        result: &mut TestRunResult,
    ) -> anyhow::Result<bool> {
        info!(
            "beginning the proper load testing. going to send at rate {}/s for {}",
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
            if !self
                .send_test_packet_batch(batch_size, egress, result)
                .await?
            {
                return Ok(false);
            }

            sent += batch_size;
            // update send count each batch
            result.set_packets_sent(sent);
        }

        if sent < total_packets {
            warn!(
                "did not manage to send all required packets within the sending window. sent {sent}/{total_packets}"
            );
        }
        Ok(true)
    }

    /// Drains all received packets from `processor` (waiting up to `waiting_duration` for
    /// stragglers), deduplicates by ID, computes RTT statistics, and populates `result`.
    async fn collect_test_results(
        &self,
        processor: &mut MixnetPacketProcessor,
        result: &mut TestRunResult,
    ) {
        // drain whatever arrived immediately, then wait for stragglers
        let mut received = processor.all_available();
        if received.len() < result.packets_sent {
            let deadline = sleep(self.config.waiting_duration);
            pin!(deadline);
            loop {
                tokio::select! {
                    _ = &mut deadline => break,
                    next = processor.next_packet() => {
                        received.push(next);
                        if received.len() >= result.packets_sent {
                            break;
                        }
                    }
                }
            }
        }

        // deduplicate by packet ID
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
                result.set_received_duplicates();
            }
        }

        let latencies = valid_received
            .values()
            .map(|p| self.packet_latency(*p))
            .collect::<Vec<_>>();

        result.set_packets_received(valid_received.len());
        result.set_packets_statistics(LatencyDistribution::compute(&latencies));
    }

    // only return error on critical, agent-level failures — not on tested node issues
    pub(crate) async fn run_stress_test(&mut self) -> anyhow::Result<TestRunResult> {
        let mut result = TestRunResult::new_empty();

        // 1. establish the egress connection — abort immediately if it fails
        let mut egress = match self.establish_egress_connection().await {
            Ok(conn) => conn,
            Err(err) => {
                result.set_error(
                    err.context("failed to establish egress node connection")
                        .to_string(),
                );
                return Ok(result);
            }
        };

        // 2. spawn the mixnet packet listener that forwards received packets to the processor
        let mut processor = self.build_packet_processor();
        let shutdown_token = ShutdownToken::new();
        let mut listener = self
            .build_mixnet_listener(processor.sender(), shutdown_token.clone())
            .await?;
        let listener_join = tokio::spawn(async move { listener.run().await });

        // 3. probe: send a single packet to confirm the node responds
        if !self
            .send_connectivity_probe(&mut egress, &mut processor, &mut result)
            .await?
        {
            return Ok(result);
        }

        // 4. probe: replay the packet to verify bloomfilter bypass is configured
        if self.config.reuse_header
            && !self
                .send_bloomfilter_probe(&mut egress, &mut processor, &mut result)
                .await?
        {
            result.set_egress_connection_statistics(egress.connection_statistics);
            return Ok(result);
        }

        // 5. stress test: send packets at the target rate for the configured duration
        self.send_load_test(&mut egress, &mut result).await?;

        // 6. collect and summarise results
        self.collect_test_results(&mut processor, &mut result).await;

        // 7. finally add missing stats
        shutdown_token.cancel();
        let mixnet_listener = listener_join.await?;
        let ingress_noise = mixnet_listener
            .last_noise_handshake_duration
            .context("missing ingress noise duration after completing entire test run!")?;

        result.set_ingress_noise_handshake(ingress_noise);
        result.set_egress_connection_statistics(egress.connection_statistics);

        Ok(result)
    }
}
