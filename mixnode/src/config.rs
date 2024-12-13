// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

// 'RTT MEASUREMENT'
const DEFAULT_PACKETS_PER_NODE: usize = 100;
const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_millis(5000);
const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
const DEFAULT_BATCH_SIZE: usize = 50;
const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);

// 'DEBUG'
const DEFAULT_NODE_STATS_LOGGING_DELAY: Duration = Duration::from_millis(60_000);
const DEFAULT_NODE_STATS_UPDATING_DELAY: Duration = Duration::from_millis(30_000);
const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_millis(300_000);
const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 2000;

#[derive(Debug, PartialEq)]
pub struct Config {
    pub host: Host,

    pub http: Http,

    pub mixnode: MixNode,

    pub verloc: Verloc,

    pub debug: Debug,
}

impl Config {
    pub fn externally_loaded(
        host: impl Into<Host>,
        http: impl Into<Http>,
        mixnode: impl Into<MixNode>,
        verloc: impl Into<Verloc>,
        debug: impl Into<Debug>,
    ) -> Self {
        Config {
            host: host.into(),
            http: http.into(),
            mixnode: mixnode.into(),
            verloc: verloc.into(),
            debug: debug.into(),
        }
    }

    // builder methods
    pub fn with_custom_nym_apis(mut self, nym_api_urls: Vec<Url>) -> Self {
        self.mixnode.nym_api_urls = nym_api_urls;
        self
    }

    pub fn with_listening_address(mut self, listening_address: IpAddr) -> Self {
        self.mixnode.listening_address = listening_address;

        let http_port = self.http.bind_address.port();
        self.http.bind_address = SocketAddr::new(listening_address, http_port);

        self
    }

    pub fn with_mix_port(mut self, port: u16) -> Self {
        self.mixnode.mix_port = port;
        self
    }

    pub fn with_verloc_port(mut self, port: u16) -> Self {
        self.mixnode.verloc_port = port;
        self
    }

    pub fn with_http_api_port(mut self, port: u16) -> Self {
        let http_ip = self.http.bind_address.ip();
        self.http.bind_address = SocketAddr::new(http_ip, port);
        self
    }

    pub fn get_nym_api_endpoints(&self) -> Vec<Url> {
        self.mixnode.nym_api_urls.clone()
    }

    pub fn with_metrics_key(mut self, metrics_key: String) -> Self {
        self.http.metrics_key = Some(metrics_key);
        self
    }

    pub fn metrics_key(&self) -> Option<&String> {
        self.http.metrics_key.as_ref()
    }
}

// TODO: this is very much a WIP. we need proper ssl certificate support here
#[derive(Debug, PartialEq)]
pub struct Host {
    /// Ip address(es) of this host, such as 1.1.1.1 that external clients will use for connections.
    pub public_ips: Vec<IpAddr>,

    /// Optional hostname of this node, for example nymtech.net.
    // TODO: this is temporary. to be replaced by pulling the data directly from the certs.
    pub hostname: Option<String>,
}

impl Host {
    pub fn validate(&self) -> bool {
        if self.public_ips.is_empty() {
            return false;
        }

        true
    }
}

#[derive(Debug, PartialEq)]
pub struct Http {
    /// Socket address this node will use for binding its http API.
    /// default: `0.0.0.0:8000`
    pub bind_address: SocketAddr,

    /// Path to assets directory of custom landing page of this node.
    pub landing_page_assets_path: Option<PathBuf>,

    pub metrics_key: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct MixNode {
    /// Version of the mixnode for which this configuration was created.
    pub version: String,

    /// ID specifies the human readable ID of this particular mixnode.
    pub id: String,

    /// Address to which this mixnode will bind to and will be listening for packets.
    pub listening_address: IpAddr,

    /// Port used for listening for all mixnet traffic.
    /// (default: 1789)
    pub mix_port: u16,

    /// Port used for listening for verloc traffic.
    /// (default: 1790)
    pub verloc_port: u16,

    /// Addresses to nym APIs from which the node gets the view of the network.
    pub nym_api_urls: Vec<Url>,
}

#[derive(Debug, PartialEq)]
pub struct Verloc {
    /// Specifies number of echo packets sent to each node during a measurement run.
    pub packets_per_node: usize,

    /// Specifies maximum amount of time to wait for the connection to get established.
    pub connection_timeout: Duration,

    /// Specifies maximum amount of time to wait for the reply packet to arrive before abandoning the test.
    pub packet_timeout: Duration,

    /// Specifies delay between subsequent test packets being sent (after receiving a reply).
    pub delay_between_packets: Duration,

    /// Specifies number of nodes being tested at once.
    pub tested_nodes_batch_size: usize,

    /// Specifies delay between subsequent test runs.
    pub testing_interval: Duration,

    /// Specifies delay between attempting to run the measurement again if the previous run failed
    /// due to being unable to get the list of nodes.
    pub retry_timeout: Duration,
}

impl Default for Verloc {
    fn default() -> Self {
        Verloc {
            packets_per_node: DEFAULT_PACKETS_PER_NODE,
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            packet_timeout: DEFAULT_PACKET_TIMEOUT,
            delay_between_packets: DEFAULT_DELAY_BETWEEN_PACKETS,
            tested_nodes_batch_size: DEFAULT_BATCH_SIZE,
            testing_interval: DEFAULT_TESTING_INTERVAL,
            retry_timeout: DEFAULT_RETRY_TIMEOUT,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Debug {
    /// Delay between each subsequent node statistics being logged to the console
    pub node_stats_logging_delay: Duration,

    /// Delay between each subsequent node statistics being updated
    pub node_stats_updating_delay: Duration,

    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    pub packet_forwarding_initial_backoff: Duration,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    pub packet_forwarding_maximum_backoff: Duration,

    /// Timeout for establishing initial connection when trying to forward a sphinx packet.
    pub initial_connection_timeout: Duration,

    /// Maximum number of packets that can be stored waiting to get sent to a particular connection.
    pub maximum_connection_buffer_size: usize,

    /// Specifies whether the mixnode should be using the legacy framing for the sphinx packets.
    // it's set to true by default. The reason for that decision is to preserve compatibility with the
    // existing nodes whilst everyone else is upgrading and getting the code for handling the new field.
    // It shall be disabled in the subsequent releases.
    pub use_legacy_framed_packet_version: bool,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            node_stats_logging_delay: DEFAULT_NODE_STATS_LOGGING_DELAY,
            node_stats_updating_delay: DEFAULT_NODE_STATS_UPDATING_DELAY,
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            maximum_connection_buffer_size: DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            use_legacy_framed_packet_version: false,
        }
    }
}
