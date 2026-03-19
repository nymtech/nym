// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::egress_connection::EgressConnection;
use crate::agent::listener::MixnetListener;
use crate::agent::processor::MixnetPacketProcessor;
use crate::agent::sphinx_helpers::{TestPacketHeader, create_test_sphinx_packet_header};
use crate::agent::test_packet::TestPacketContent;
use humantime::format_duration;
use nym_crypto::asymmetric::x25519;
use nym_noise::config::{NoiseConfig, NoiseNetworkView, NoiseNode};
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_params::SphinxKeyRotation;
use nym_task::ShutdownToken;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::pin;
use tokio::time::{Instant, sleep};
use tracing::{info, warn};

mod egress_connection;
pub(crate) mod listener;
mod processor;
pub(crate) mod receiver;
mod sphinx_helpers;
pub(crate) mod test_packet;

/// Configuration for the [`NetworkMonitorAgent`], controlling packet sending behaviour during a test run.
pub(crate) struct Config {
    /// How long the agent should be sending test packets with the specified rate.
    pub(crate) sending_duration: Duration,

    /// How long the agent will wait to receive any leftover packets after finishing sending.
    pub(crate) waiting_duration: Duration,

    /// How long the target node should delay the packet (i.e. the sphinx delay)
    pub(crate) packet_delay: Duration,

    /// Timeout for establishing the egress connection to the node under test.
    pub(crate) egress_connection_timeout: Duration,

    /// Timeout for the completing the noise handshake.
    pub(crate) noise_handshake_timeout: Duration,

    /// Number of packets sent in a single batch per unit time.
    pub(crate) sending_batch_size: usize,

    /// Target rate of packets (per second) to be sent.
    pub(crate) target_rate: usize,

    /// Whether the agent should reuse the same header for all packets, and consequently replay them.
    pub(crate) reuse_header: bool,

    /// Address of the mixnet listener on this agent
    pub(crate) mixnet_address: SocketAddr,
}

impl Config {
    pub(crate) fn expected_packets(&self) -> usize {
        (self.target_rate as f32 * self.sending_duration.as_secs_f32()).floor() as usize
    }

    pub(crate) fn batches(&self) -> usize {
        self.expected_packets().div_ceil(self.sending_batch_size)
    }

    pub(crate) fn batch_interval(&self) -> Duration {
        Duration::from_secs_f64(self.sending_batch_size as f64 / self.target_rate as f64)
    }
}

pub(crate) struct TestedNodeDetails {
    pub(crate) address: SocketAddr,

    pub(crate) noise_key: x25519::PublicKey,

    /// Key rotation associated with the current sphinx key of the node
    pub(crate) key_rotation: SphinxKeyRotation,

    pub(crate) sphinx_key: x25519::PublicKey,
}

impl TestedNodeDetails {
    pub(crate) fn as_sphinx_node(&self) -> anyhow::Result<nym_sphinx_types::Node> {
        Ok(nym_sphinx_types::Node::new(
            NymNodeRoutingAddress::from(self.address).try_into()?,
            self.sphinx_key.into(),
        ))
    }

    pub(crate) fn as_noise_node(&self) -> NoiseNode {
        NoiseNode::new_from_inner_key(self.noise_key, 1, true)
    }
}

pub(crate) struct NetworkMonitorAgent {
    config: Config,

    noise_key: Arc<x25519::KeyPair>,

    sphinx_key: x25519::PrivateKey,

    tested_node: TestedNodeDetails,
}

pub(crate) struct TestRunResult {
    //
}

impl NetworkMonitorAgent {
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
        Ok(nym_sphinx_types::Node::new(
            NymNodeRoutingAddress::from(self.config.mixnet_address).try_into()?,
            self.sphinx_key.public_key().into(),
        ))
    }

    fn create_test_sphinx_packet_header(&self) -> anyhow::Result<TestPacketHeader> {
        // we don't want any delays
        // and the packet route is test node -> this client
        let route = vec![self.tested_node.as_sphinx_node()?, self.as_sphinx_node()?];
        let delay = self.config.packet_delay;
        create_test_sphinx_packet_header(route, delay)
    }

    pub(crate) async fn run_stress_test(&self) -> anyhow::Result<TestRunResult> {
        let noise_config = self.noise_config();

        // 1. establish the connection - if it fails, there's no point in continuing
        let mut egress_connection = match EgressConnection::establish(
            self.config.mixnet_address,
            self.config.egress_connection_timeout,
            self.tested_node.key_rotation,
            &noise_config,
        )
        .await
        {
            Ok(conn) => conn,
            Err(err) => {
                todo!()
            }
        };

        let mut id = 0;
        let test_header = self.create_test_sphinx_packet_header()?;

        let mut processor =
            MixnetPacketProcessor::new(test_header.clone(), self.config.waiting_duration);
        let shutdown_token = ShutdownToken::new();
        let mut listener = MixnetListener::new(
            self.config.mixnet_address,
            self.tested_node.address,
            noise_config,
            processor.sender(),
            shutdown_token.clone(),
        );

        // 2. spawn the mixnet packet listener that forwards any received packets to the processor
        let listener_join = tokio::spawn(async move { listener.run().await });

        let content = TestPacketContent::new(id);
        let test_packet = test_header.create_test_packet(content)?;

        // 3. send a single packet to see if the node is even going to respond to it
        info!("sending initial packet");
        egress_connection.send_packet(test_packet).await?;
        match processor.next_packet().await {
            Ok(res) => {
                info!(
                    "received packet {} after {}",
                    res.id,
                    humantime::format_duration(res.rtt)
                )
            }
            Err(err) => todo!(),
        }

        // 4. send it again to check if the node is configured correctly for testing
        // (i.e. whether the agent can bypass the bloomfilter)
        if self.config.reuse_header {
            info!("repeating the packet to check bloomfilter bypass configuration");

            id += 1;
            let content = TestPacketContent::new(id);
            let test_packet = test_header.create_test_packet(content)?;
            egress_connection.send_packet(test_packet).await?;

            match processor.next_packet().await {
                Ok(res) => {
                    info!(
                        "received packet {} after {}",
                        res.id,
                        humantime::format_duration(res.rtt)
                    )
                }
                Err(err) => todo!(),
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

            let mut batch = Vec::with_capacity(batch_size);
            for _ in 0..batch_size {
                id += 1;
                let content = TestPacketContent::new(id);
                batch.push(test_header.create_test_packet(content)?);
            }

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

        todo!()
    }
}
