// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::NodeTesterConfig;
use crate::agent::result::{LatencyDistribution, TestRunResult};
use crate::agent::tested_node::TestedNodeDetails;
use crate::egress_connection::EgressConnection;
use crate::listener::MixnetListener;
use crate::listener::received::MixnetPacketsSender;
use crate::processor::{MixnetPacketProcessor, ProcessedPacket};
use crate::sphinx_helpers::{
    as_sphinx_node, build_test_sphinx_packet, create_test_sphinx_packet_header,
};
use crate::test_packet::{TestPacketContent, TestPacketHeader};
use anyhow::Context;
use humantime::format_duration;
use nym_crypto::asymmetric::x25519;
use nym_noise::config::{NoiseConfig, NoiseNetworkView};
use nym_sphinx_types::SphinxPacket;
use nym_task::ShutdownToken;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::pin;
use tokio::time::{Instant, sleep};
use tracing::{debug, error, info, warn};

/// The core component responsible for executing a stress-test run against a single node.
///
/// A test run proceeds in five ordered steps (see [`run_stress_test`](Self::run_stress_test)):
///
/// 1. Establish an outbound (egress) Noise-encrypted TCP connection to the node.
/// 2. Bind a local TCP listener (ingress) that receives sphinx packets the node sends back.
/// 3. Send a single probe packet to verify basic connectivity and record baseline latency.
/// 4. Replay the same packet (when `reuse_header` is enabled) to confirm the node's
///    bloomfilter bypass is correctly configured.
/// 5. Send packets at the configured rate for the configured duration, then collect and
///    summarise the results.
///
/// Only critical failures (e.g. failing to bind a port) are returned as
/// `Err`; node-level failures (e.g. the node not responding) are captured inside the
/// returned [`TestRunResult`] so the caller can still inspect partial data.
pub(crate) struct NodeStressTester {
    /// Tester configuration controlling rates, timeouts, and addressing.
    config: NodeTesterConfig,

    /// Monotonically increasing counter embedded in each outgoing packet as its ID.
    packet_counter: u64,

    /// Pre-built sphinx packet header reused across all packets when `config.reuse_header`
    /// is set. Allows the node's bloomfilter bypass to be exercised. `None` means a fresh
    /// header is built for every packet.
    reusable_test_header: Option<TestPacketHeader>,

    /// The tester's own Noise key pair, used to authenticate the egress connection.
    noise_key: Arc<x25519::KeyPair>,

    /// An ephemeral sphinx key pair generated at construction time. Used both to build the
    /// return-route sphinx header (so packets come back to this tester) and to decrypt
    /// returning packets when `reuse_header` is disabled.
    sphinx_key: Arc<x25519::KeyPair>,

    /// Identity and addressing information for the node being tested.
    tested_node: TestedNodeDetails,
}

impl NodeStressTester {
    /// Creates a new tester, loading the Noise private key from `noise_key_path` and
    /// generating a fresh ephemeral sphinx key. If `config.reuse_header` is set, the
    /// sphinx packet header is pre-built here so it can be reused across all test packets.
    pub(crate) fn new(
        config: NodeTesterConfig,
        noise_key: Arc<x25519::KeyPair>,
        tested_node: TestedNodeDetails,
    ) -> anyhow::Result<Self> {
        info!("using the following tester config");
        info!("{config:#?}");

        info!("testing the following node");
        info!("{tested_node:#?}");

        let sphinx_key = x25519::PrivateKey::new(&mut OsRng);

        let reusable_test_header = if config.reuse_header {
            // Route: tested node → this agent (so packets come back to us).
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
            noise_key,
            sphinx_key: Arc::new(sphinx_key.into()),
            tested_node,
        })
    }

    /// Opens the outbound Noise-encrypted TCP connection to the node under test.
    async fn establish_egress_connection(&self) -> anyhow::Result<EgressConnection> {
        EgressConnection::establish(
            self.tested_node.address,
            self.config.egress_connection_timeout,
            self.tested_node.key_rotation,
            &self.noise_config(),
        )
        .await
    }

    /// Constructs the [`MixnetPacketProcessor`] used to decode and time-stamp returning packets.
    /// When a reusable header is available it is used for decryption; otherwise the tester's
    /// sphinx private key is used directly.
    fn build_packet_processor(&self) -> MixnetPacketProcessor {
        let packet_recovery = match &self.reusable_test_header {
            Some(header) => header.clone().into(),
            None => self.sphinx_key.clone().into(),
        };
        MixnetPacketProcessor::new(packet_recovery, self.config.waiting_duration)
    }

    /// Binds the local TCP listener and wraps it in a [`MixnetListener`] that will forward
    /// decoded packets to `received_sender`.
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

    /// Builds a [`NoiseConfig`] that contains the default configuration for the protocol
    /// and the key associated with the tested node to accept its connection.
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

    /// Returns a sphinx node representation of this tester's own mixnet listener address,
    /// used as the final hop in the packet route so packets are delivered back here.
    fn as_sphinx_node(&self) -> nym_sphinx_types::Node {
        as_sphinx_node(self.config.mixnet_address, *self.sphinx_key.public_key())
    }

    /// Builds the next test sphinx packet, incrementing the internal packet counter.
    /// Reuses the pre-built header when available; otherwise builds a fresh header and
    /// encrypts it with a new sphinx key each time.
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

    /// Builds a batch of `batch_size` test sphinx packets with consecutive IDs.
    fn create_packet_batch(&mut self, batch_size: usize) -> anyhow::Result<Vec<SphinxPacket>> {
        let mut packets = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let packet = self.create_test_sphinx_packet()?;
            packets.push(packet);
        }
        Ok(packets)
    }

    /// Computes the network latency for a received packet by subtracting the configured
    /// sphinx delay from its measured round-trip time.
    fn packet_latency(&self, received: ProcessedPacket) -> Duration {
        received.rtt - self.config.packet_delay
    }

    /// Creates and sends a single test sphinx packet over `egress`.
    /// On send failure, records an error on `result` and returns `false`.
    async fn send_test_packet(
        &mut self,
        egress: &mut EgressConnection,
        result: &mut TestRunResult,
    ) -> anyhow::Result<bool> {
        let packet = self
            .create_test_sphinx_packet()
            .context("sphinx packet creation failure!")?;
        if let Err(err) = egress.send_packet(packet).await {
            result.set_error(format!("{:#}", err.context("failed to send test packet")));
            return Ok(false);
        };
        Ok(true)
    }

    /// Creates and sends a batch of `batch_size` test packets over `egress`.
    /// On send failure, records an error on `result` and returns `false`.
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
            result.set_error(format!("{:#}", err.context("failed to send test packet")));
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
                result.set_error(format!(
                    "{:#}",
                    err.context("failed to receive a valid initial packet back")
                ));
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
                result.set_error(format!(
                    "{:#}",
                    err.context("failed to receive a valid secondary packet back - the node might not have a working chain subscriber (or the agent might be misconfigured)"))
                );
                Ok(false)
            }
        }
    }

    /// Sends packets at the configured rate for the configured duration.
    /// Dispatches one batch every `batch_interval` seconds; if the egress falls behind,
    /// ticks are delayed rather than bunched up to avoid unintended bursts.
    /// Updates `result.packets_sent` after every batch and returns `false` on send failure.
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
            // update send count after each batch so partial results are visible on early exit
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

        // deduplicate by packet ID; duplicates indicate possible node misbehaviour
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

        let received_count = valid_received.len();
        result.set_packets_received(received_count);
        result.set_packets_statistics(LatencyDistribution::compute(&latencies));

        info!(
            sent = result.packets_sent,
            received = received_count,
            recv_pct = format!("{:.1}%", result.received_percentage()),
            "load test complete"
        );
    }

    /// Runs a full stress-test against the configured node and returns the collected results.
    ///
    /// Only returns `Err` for critical failures (e.g. unable to bind the listener
    /// port). Node-level failures (no response, bloomfilter misconfiguration, etc.) are
    /// recorded inside the returned [`TestRunResult`] so the caller always gets partial data.
    pub(crate) async fn run_stress_test(&mut self) -> anyhow::Result<TestRunResult> {
        let mut result = TestRunResult::new_empty();

        // 1. establish the egress connection — abort immediately if it fails
        let mut egress = match self.establish_egress_connection().await {
            Ok(conn) => conn,
            Err(err) => {
                result.set_error(format!(
                    "{:#}",
                    err.context("failed to establish egress node connection")
                ));
                return Ok(result);
            }
        };

        // 2. spawn the mixnet packet listener that forwards received packets to the processor
        let mut processor = self.build_packet_processor();
        let shutdown_token = ShutdownToken::new();
        let listener = self
            .build_mixnet_listener(processor.sender(), shutdown_token.clone())
            .await?;
        let listener_join = tokio::spawn(async move { listener.run().await });

        // 3. probe: send a single packet to confirm the node responds
        if !self
            .send_connectivity_probe(&mut egress, &mut processor, &mut result)
            .await?
        {
            shutdown_token.cancel();
            let _ = listener_join.await?;
            return Ok(result);
        }

        // 4. probe: replay the packet to verify bloomfilter bypass is configured
        if self.config.reuse_header
            && !self
                .send_bloomfilter_probe(&mut egress, &mut processor, &mut result)
                .await?
        {
            shutdown_token.cancel();
            let mixnet_listener = listener_join.await?;
            let ingress_noise = mixnet_listener
                .last_noise_handshake_duration
                .context("missing ingress noise duration after completing entire test run!")?;

            result.set_ingress_noise_handshake(ingress_noise);
            result.set_egress_connection_statistics(egress.connection_statistics);
            return Ok(result);
        }

        // 5. stress test: send packets at the target rate for the configured duration
        self.send_load_test(&mut egress, &mut result).await?;

        // 6. collect and summarise results
        self.collect_test_results(&mut processor, &mut result).await;

        // 7. shut down the listener and harvest its stats
        info!("shutting down the mixnet listener");
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
