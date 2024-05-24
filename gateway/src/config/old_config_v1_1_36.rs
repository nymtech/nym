// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::paths::GatewayPaths;
use nym_bin_common::logging::LoggingSettings;
use nym_config::{
    must_get_home, read_config_from_toml_file, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, NYM_DIR,
};
use nym_network_defaults::WG_PORT;
use serde::{Deserialize, Deserializer, Serialize};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

use super::persistence::paths::KeysPaths;
use super::{Config, Debug, Gateway, Host, Http, NetworkRequester};

const DEFAULT_GATEWAYS_DIR: &str = "gateways";

// 'DEBUG'
// where applicable, the below are defined in milliseconds
const DEFAULT_PRESENCE_SENDING_DELAY: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_millis(300_000);
const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 2000;

const DEFAULT_STORED_MESSAGE_FILENAME_LENGTH: u16 = 16;
const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;

fn de_maybe_port<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    let port = u16::deserialize(deserializer)?;
    if port == 0 {
        Ok(None)
    } else {
        Ok(Some(port))
    }
}

fn de_maybe_path<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    let path = PathBuf::deserialize(deserializer)?;
    if path.as_os_str().is_empty() {
        Ok(None)
    } else {
        Ok(Some(path))
    }
}

/// Derive default path to gateway's config directory.
/// It should get resolved to `$HOME/.nym/gateways/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_GATEWAYS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to gateways's config file.
/// It should get resolved to `$HOME/.nym/gateways/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_36 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    pub host: Host,

    #[serde(default)]
    pub http: Http,

    pub gateway: GatewayV1_1_36,

    #[serde(default)]
    // currently not really used for anything useful
    pub wireguard: WireguardV1_1_36,

    pub storage_paths: GatewayPathsV1_1_36,

    pub network_requester: NetworkRequesterV1_1_36,

    #[serde(default)]
    pub ip_packet_router: IpPacketRouterV1_1_36,

    #[serde(default)]
    pub logging: LoggingSettingsV1_1_36,

    #[serde(default)]
    pub debug: DebugV1_1_36,
}

impl ConfigV1_1_36 {
    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        read_config_from_toml_file(default_config_filepath(id))
    }
}

impl From<ConfigV1_1_36> for Config {
    fn from(value: ConfigV1_1_36) -> Self {
        Self {
            save_path: value.save_path,
            host: value.host,
            http: value.http,
            gateway: Gateway {
                version: value.gateway.version,
                id: value.gateway.id,
                only_coconut_credentials: value.gateway.only_coconut_credentials,
                listening_address: value.gateway.listening_address,
                mix_port: value.gateway.mix_port,
                clients_port: value.gateway.clients_port,
                clients_wss_port: value.gateway.clients_wss_port,
                enabled_statistics: value.gateway.enabled_statistics,
                statistics_service_url: value.gateway.statistics_service_url,
                nym_api_urls: value.gateway.nym_api_urls,
                nyxd_urls: value.gateway.nyxd_urls,
                cosmos_mnemonic: value.gateway.cosmos_mnemonic,
            },
            storage_paths: GatewayPaths {
                keys: KeysPaths {
                    private_identity_key_file: value.storage_paths.keys.private_identity_key_file,
                    public_identity_key_file: value.storage_paths.keys.public_identity_key_file,
                    private_sphinx_key_file: value.storage_paths.keys.private_sphinx_key_file,
                    public_sphinx_key_file: value.storage_paths.keys.public_sphinx_key_file,
                },
                clients_storage: value.storage_paths.clients_storage,
                network_requester_config: value.storage_paths.network_requester_config,
                // \/ ADDED
                ip_packet_router_config: Default::default(),
                // /\ ADDED
            },
            network_requester: NetworkRequester {
                enabled: value.network_requester.enabled,
            },
            // \/ ADDED
            ip_packet_router: Default::default(),
            // /\ ADDED
            logging: LoggingSettings {
                // no fields (yet)
            },
            debug: Debug {
                packet_forwarding_initial_backoff: value.debug.packet_forwarding_initial_backoff,
                packet_forwarding_maximum_backoff: value.debug.packet_forwarding_maximum_backoff,
                initial_connection_timeout: value.debug.initial_connection_timeout,
                maximum_connection_buffer_size: value.debug.maximum_connection_buffer_size,
                presence_sending_delay: value.debug.presence_sending_delay,
                stored_messages_filename_length: value.debug.stored_messages_filename_length,
                message_retrieval_limit: value.debug.message_retrieval_limit,
                use_legacy_framed_packet_version: value.debug.use_legacy_framed_packet_version,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GatewayV1_1_36 {
    /// Version of the gateway for which this configuration was created.
    pub version: String,

    /// ID specifies the human readable ID of this particular gateway.
    pub id: String,

    /// Indicates whether this gateway is accepting only coconut credentials for accessing the
    /// the mixnet, or if it also accepts non-paying clients
    #[serde(default)]
    pub only_coconut_credentials: bool,

    /// Address to which this mixnode will bind to and will be listening for packets.
    pub listening_address: IpAddr,

    /// Port used for listening for all mixnet traffic.
    /// (default: 1789)
    pub mix_port: u16,

    /// Port used for listening for all client-related traffic.
    /// (default: 9000)
    pub clients_port: u16,

    /// If applicable, announced port for listening for secure websocket client traffic.
    /// (default: None)
    #[serde(deserialize_with = "de_maybe_port")]
    pub clients_wss_port: Option<u16>,

    /// Whether gateway collects and sends anonymized statistics
    pub enabled_statistics: bool,

    /// Domain address of the statistics service
    pub statistics_service_url: Url,

    /// Addresses to APIs from which the node gets the view of the network.
    #[serde(alias = "validator_api_urls")]
    pub nym_api_urls: Vec<Url>,

    /// Addresses to validators which the node uses to check for double spending of ERC20 tokens.
    #[serde(alias = "validator_nymd_urls")]
    pub nyxd_urls: Vec<Url>,

    /// Mnemonic of a cosmos wallet used in checking for double spending.
    // #[deprecated(note = "move to storage")]
    // TODO: I don't think this should be stored directly in the config...
    pub cosmos_mnemonic: bip39::Mnemonic,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct WireguardV1_1_36 {
    /// Specifies whether the wireguard service is enabled on this node.
    pub enabled: bool,

    /// Socket address this node will use for binding its wireguard interface.
    /// default: `0.0.0.0:51820`
    pub bind_address: SocketAddr,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_port: u16,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard.
    /// The maximum value for IPv4 is 32 and for IPv6 is 128
    pub private_network_prefix: u8,

    /// Paths for wireguard keys, client registries, etc.
    pub storage_paths: WireguardPathsV1_1_36,
}

impl Default for WireguardV1_1_36 {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), WG_PORT),
            announced_port: WG_PORT,
            storage_paths: WireguardPathsV1_1_36 {},
            private_network_prefix: 16,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardPathsV1_1_36 {
    // pub keys:
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayPathsV1_1_36 {
    pub keys: KeysPathsV1_1_36,

    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys and available client bandwidths.
    #[serde(alias = "persistent_storage")]
    pub clients_storage: PathBuf,

    /// Path to the configuration of the embedded network requester.
    #[serde(deserialize_with = "de_maybe_path")]
    pub network_requester_config: Option<PathBuf>,
    // pub node_description: PathBuf,

    // pub cosmos_bip39_mnemonic: PathBuf,
    /// Path to the configuration of the embedded ip packet router.
    #[serde(deserialize_with = "de_maybe_path")]
    pub ip_packet_router_config: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct KeysPathsV1_1_36 {
    /// Path to file containing private identity key.
    pub private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    pub public_identity_key_file: PathBuf,

    /// Path to file containing private sphinx key.
    pub private_sphinx_key_file: PathBuf,

    /// Path to file containing public sphinx key.
    pub public_sphinx_key_file: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct NetworkRequesterV1_1_36 {
    /// Specifies whether network requester service is enabled in this process.
    pub enabled: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for NetworkRequesterV1_1_36 {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSettingsV1_1_36 {
    // well, we need to implement something here at some point...
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct DebugV1_1_36 {
    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_initial_backoff: Duration,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_maximum_backoff: Duration,

    /// Timeout for establishing initial connection when trying to forward a sphinx packet.
    #[serde(with = "humantime_serde")]
    pub initial_connection_timeout: Duration,

    /// Maximum number of packets that can be stored waiting to get sent to a particular connection.
    pub maximum_connection_buffer_size: usize,

    /// Delay between each subsequent presence data being sent.
    #[serde(with = "humantime_serde")]
    pub presence_sending_delay: Duration,

    /// Length of filenames for new client messages.
    pub stored_messages_filename_length: u16,

    /// Number of messages from offline client that can be pulled at once from the storage.
    pub message_retrieval_limit: i64,

    /// Specifies whether the mixnode should be using the legacy framing for the sphinx packets.
    // it's set to true by default. The reason for that decision is to preserve compatibility with the
    // existing nodes whilst everyone else is upgrading and getting the code for handling the new field.
    // It shall be disabled in the subsequent releases.
    pub use_legacy_framed_packet_version: bool,
}

impl Default for DebugV1_1_36 {
    fn default() -> Self {
        Self {
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            presence_sending_delay: DEFAULT_PRESENCE_SENDING_DELAY,
            maximum_connection_buffer_size: DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            stored_messages_filename_length: DEFAULT_STORED_MESSAGE_FILENAME_LENGTH,
            message_retrieval_limit: DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            use_legacy_framed_packet_version: false,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct IpPacketRouterV1_1_36 {
    /// Specifies whether ip packet router service is enabled in this process.
    pub enabled: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for IpPacketRouterV1_1_36 {
    fn default() -> Self {
        Self { enabled: false }
    }
}
