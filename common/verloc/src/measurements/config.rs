// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::UserAgent;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use url::Url;

// by default all of those are overwritten by config data from nym-node directly
const DEFAULT_VERLOC_PORT: u16 = 1790;
const DEFAULT_PACKETS_PER_NODE: usize = 100;
const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_millis(5000);
const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
const DEFAULT_BATCH_SIZE: usize = 50;
const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);

#[derive(Clone, Debug)]
pub struct Config {
    /// Socket address of this node on which it will be listening for the measurement packets.
    pub listening_address: SocketAddr,

    /// Specifies number of echo packets sent to each node during a measurement run.
    pub packets_per_node: usize,

    /// Specifies maximum amount of time to wait for the reply packet to arrive before abandoning the test.
    pub packet_timeout: Duration,

    /// Specifies maximum amount of time to wait for the connection to get established.
    pub connection_timeout: Duration,

    /// Specifies delay between subsequent test packets being sent (after receiving a reply).
    pub delay_between_packets: Duration,

    /// Specifies number of nodes being tested at once.
    pub tested_nodes_batch_size: usize,

    /// Specifies delay between subsequent test runs.
    pub testing_interval: Duration,

    /// Specifies delay between attempting to run the measurement again if the previous run failed
    /// due to being unable to get the list of nodes.
    pub retry_timeout: Duration,

    /// URLs to the nym apis for obtaining network topology.
    pub nym_api_urls: Vec<Url>,

    /// User agent used for querying the nym-api
    pub user_agent: UserAgent,
}

impl Config {
    pub fn build(nym_api_urls: Vec<Url>, user_agent: impl Into<UserAgent>) -> ConfigBuilder {
        ConfigBuilder::new(nym_api_urls, user_agent)
    }
}

#[must_use]
pub struct ConfigBuilder(Config);

impl ConfigBuilder {
    pub fn new(nym_api_urls: Vec<Url>, user_agent: impl Into<UserAgent>) -> ConfigBuilder {
        ConfigBuilder(Config {
            // '[::]:port'
            listening_address: SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                DEFAULT_VERLOC_PORT,
            ),
            packets_per_node: DEFAULT_PACKETS_PER_NODE,
            packet_timeout: DEFAULT_PACKET_TIMEOUT,
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            delay_between_packets: DEFAULT_DELAY_BETWEEN_PACKETS,
            tested_nodes_batch_size: DEFAULT_BATCH_SIZE,
            testing_interval: DEFAULT_TESTING_INTERVAL,
            retry_timeout: DEFAULT_RETRY_TIMEOUT,
            nym_api_urls,
            user_agent: user_agent.into(),
        })
    }

    pub fn listening_address(mut self, listening_address: SocketAddr) -> Self {
        self.0.listening_address = listening_address;
        self
    }

    pub fn packets_per_node(mut self, packets_per_node: usize) -> Self {
        self.0.packets_per_node = packets_per_node;
        self
    }

    pub fn packet_timeout(mut self, packet_timeout: Duration) -> Self {
        self.0.packet_timeout = packet_timeout;
        self
    }

    pub fn connection_timeout(mut self, connection_timeout: Duration) -> Self {
        self.0.connection_timeout = connection_timeout;
        self
    }

    pub fn delay_between_packets(mut self, delay_between_packets: Duration) -> Self {
        self.0.delay_between_packets = delay_between_packets;
        self
    }

    pub fn tested_nodes_batch_size(mut self, tested_nodes_batch_size: usize) -> Self {
        self.0.tested_nodes_batch_size = tested_nodes_batch_size;
        self
    }

    pub fn testing_interval(mut self, testing_interval: Duration) -> Self {
        self.0.testing_interval = testing_interval;
        self
    }

    pub fn retry_timeout(mut self, retry_timeout: Duration) -> Self {
        self.0.retry_timeout = retry_timeout;
        self
    }

    pub fn nym_api_urls(mut self, nym_api_urls: Vec<Url>) -> Self {
        self.0.nym_api_urls = nym_api_urls;
        self
    }

    pub fn build(self) -> Config {
        // panics here are fine as those are only ever constructed at the initial setup
        assert!(
            !self.0.nym_api_urls.is_empty(),
            "at least one validator endpoint must be provided",
        );
        self.0
    }
}
