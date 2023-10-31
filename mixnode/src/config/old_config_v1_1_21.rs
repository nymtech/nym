// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::old_config_v1_1_32::{
    ConfigV1_1_32, DebugV1_1_32, MixNodeV1_1_32, VerlocV1_1_32,
};
use crate::config::persistence::paths::{KeysPaths, MixNodePaths};
use nym_bin_common::logging::LoggingSettings;
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use nym_validator_client::nyxd;
use serde::{Deserialize, Deserializer, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

const DEFAULT_MIX_LISTENING_PORT: u16 = 1789;
const DEFAULT_VERLOC_LISTENING_PORT: u16 = 1790;
const DEFAULT_HTTP_API_LISTENING_PORT: u16 = 8000;
const NYM_API: &str = "https://validator.nymtech.net/api/";
const DESCRIPTION_FILE: &str = "description.toml";

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

pub(super) fn de_ipaddr_from_maybe_str_socks_addr<'de, D>(
    deserializer: D,
) -> Result<IpAddr, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if let Ok(socket_addr) = SocketAddr::from_str(&s) {
        Ok(socket_addr.ip())
    } else {
        IpAddr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

fn bind_all_address() -> IpAddr {
    "0.0.0.0".parse().unwrap()
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_21 {
    mixnode: MixNodeV1_1_21,

    #[serde(default)]
    verloc: VerlocV1_1_21,
    #[serde(default)]
    logging: LoggingV1_1_21,
    #[serde(default)]
    debug: DebugV1_1_21,
}

impl From<ConfigV1_1_21> for ConfigV1_1_32 {
    fn from(value: ConfigV1_1_21) -> Self {
        let node_description =
            ConfigV1_1_21::default_config_directory(&value.mixnode.id).join(DESCRIPTION_FILE);

        ConfigV1_1_32 {
            mixnode: MixNodeV1_1_32 {
                version: value.mixnode.version,
                id: value.mixnode.id,
                listening_address: value.mixnode.listening_address,
                mix_port: value.mixnode.mix_port,
                verloc_port: value.mixnode.verloc_port,
                http_api_port: value.mixnode.http_api_port,
                nym_api_urls: value.mixnode.nym_api_urls,
            },
            storage_paths: MixNodePaths {
                keys: KeysPaths {
                    private_identity_key_file: value.mixnode.private_identity_key_file,
                    public_identity_key_file: value.mixnode.public_identity_key_file,
                    private_sphinx_key_file: value.mixnode.private_sphinx_key_file,
                    public_sphinx_key_file: value.mixnode.public_sphinx_key_file,
                },
                node_description,
            },
            verloc: value.verloc.into(),
            logging: value.logging.into(),
            debug: value.debug.into(),
        }
    }
}

impl MigrationNymConfig for ConfigV1_1_21 {
    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("mixnodes")
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MixNodeV1_1_21 {
    version: String,
    id: String,
    #[serde(deserialize_with = "de_ipaddr_from_maybe_str_socks_addr")]
    listening_address: IpAddr,
    announce_address: String,
    mix_port: u16,
    verloc_port: u16,
    http_api_port: u16,
    private_identity_key_file: PathBuf,
    public_identity_key_file: PathBuf,
    private_sphinx_key_file: PathBuf,
    public_sphinx_key_file: PathBuf,
    nym_api_urls: Vec<Url>,
    nym_root_directory: PathBuf,
    wallet_address: Option<nyxd::AccountId>,
}

impl Default for MixNodeV1_1_21 {
    fn default() -> Self {
        MixNodeV1_1_21 {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: "".to_string(),
            listening_address: bind_all_address(),
            announce_address: "127.0.0.1".to_string(),
            mix_port: DEFAULT_MIX_LISTENING_PORT,
            verloc_port: DEFAULT_VERLOC_LISTENING_PORT,
            http_api_port: DEFAULT_HTTP_API_LISTENING_PORT,
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
            private_sphinx_key_file: Default::default(),
            public_sphinx_key_file: Default::default(),
            nym_api_urls: vec![Url::from_str(NYM_API).expect("Invalid default API URL")],
            nym_root_directory: ConfigV1_1_21::default_root_directory(),
            wallet_address: None,
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct LoggingV1_1_21 {}

impl From<LoggingV1_1_21> for LoggingSettings {
    fn from(_value: LoggingV1_1_21) -> Self {
        LoggingSettings {}
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct VerlocV1_1_21 {
    packets_per_node: usize,
    connection_timeout: Duration,
    packet_timeout: Duration,
    delay_between_packets: Duration,
    tested_nodes_batch_size: usize,
    testing_interval: Duration,
    retry_timeout: Duration,
}

impl From<VerlocV1_1_21> for VerlocV1_1_32 {
    fn from(value: VerlocV1_1_21) -> Self {
        VerlocV1_1_32 {
            packets_per_node: value.packets_per_node,
            connection_timeout: value.connection_timeout,
            packet_timeout: value.packet_timeout,
            delay_between_packets: value.delay_between_packets,
            tested_nodes_batch_size: value.tested_nodes_batch_size,
            testing_interval: value.testing_interval,
            retry_timeout: value.retry_timeout,
        }
    }
}

impl Default for VerlocV1_1_21 {
    fn default() -> Self {
        VerlocV1_1_21 {
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

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
struct DebugV1_1_21 {
    #[serde(with = "humantime_serde")]
    node_stats_logging_delay: Duration,

    #[serde(with = "humantime_serde")]
    node_stats_updating_delay: Duration,

    #[serde(with = "humantime_serde")]
    packet_forwarding_initial_backoff: Duration,

    #[serde(with = "humantime_serde")]
    packet_forwarding_maximum_backoff: Duration,

    #[serde(with = "humantime_serde")]
    initial_connection_timeout: Duration,

    maximum_connection_buffer_size: usize,

    use_legacy_framed_packet_version: bool,
}

impl From<DebugV1_1_21> for DebugV1_1_32 {
    fn from(value: DebugV1_1_21) -> Self {
        DebugV1_1_32 {
            node_stats_logging_delay: value.node_stats_logging_delay,
            node_stats_updating_delay: value.node_stats_updating_delay,
            packet_forwarding_initial_backoff: value.packet_forwarding_initial_backoff,
            packet_forwarding_maximum_backoff: value.packet_forwarding_maximum_backoff,
            initial_connection_timeout: value.initial_connection_timeout,
            maximum_connection_buffer_size: value.maximum_connection_buffer_size,
            use_legacy_framed_packet_version: value.use_legacy_framed_packet_version,
        }
    }
}

impl Default for DebugV1_1_21 {
    fn default() -> Self {
        DebugV1_1_21 {
            node_stats_logging_delay: DEFAULT_NODE_STATS_LOGGING_DELAY,
            node_stats_updating_delay: DEFAULT_NODE_STATS_UPDATING_DELAY,
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            maximum_connection_buffer_size: DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            use_legacy_framed_packet_version: true,
        }
    }
}
