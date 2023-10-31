// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_config::{
    must_get_home, read_config_from_toml_file, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, NYM_DIR,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

use super::old_config_v1_1_31::{
    ConfigV1_1_31, DebugV1_1_31, GatewayPathsV1_1_31, GatewayV1_1_31, KeysPathsV1_1_31,
    LoggingSettingsV1_1_31, NetworkRequesterV1_1_31,
};

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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeysPathsV1_1_29 {
    pub private_identity_key_file: PathBuf,
    pub public_identity_key_file: PathBuf,
    pub private_sphinx_key_file: PathBuf,
    pub public_sphinx_key_file: PathBuf,
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayPathsV1_1_29 {
    pub keys: KeysPathsV1_1_29,

    #[serde(alias = "persistent_storage")]
    pub clients_storage: PathBuf,

    #[serde(deserialize_with = "de_maybe_path")]
    pub network_requester_config: Option<PathBuf>,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSettingsV1_1_29 {}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_29 {
    #[serde(skip)]
    pub save_path: Option<PathBuf>,

    pub gateway: GatewayV1_1_29,

    pub storage_paths: GatewayPathsV1_1_29,

    pub network_requester: NetworkRequesterV1_1_29,

    #[serde(default)]
    pub logging: LoggingSettingsV1_1_29,

    #[serde(default)]
    pub debug: DebugV1_1_29,
}

impl ConfigV1_1_29 {
    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        read_config_from_toml_file(default_config_filepath(id))
    }
}

impl From<ConfigV1_1_29> for ConfigV1_1_31 {
    fn from(value: ConfigV1_1_29) -> Self {
        ConfigV1_1_31 {
            save_path: value.save_path,

            // \/ ADDED
            host: nym_node::config::Host {
                // this is a very bad default!
                public_ips: vec![value.gateway.listening_address],
                hostname: None,
            },
            // /\ ADDED

            // \/ ADDED
            http: Default::default(),
            // /\ ADDED
            gateway: GatewayV1_1_31 {
                version: value.gateway.version,
                id: value.gateway.id,
                only_coconut_credentials: value.gateway.only_coconut_credentials,
                listening_address: value.gateway.listening_address,
                mix_port: value.gateway.mix_port,
                clients_port: value.gateway.clients_port,

                // \/ ADDED
                clients_wss_port: None,
                // /\ ADDED
                enabled_statistics: value.gateway.enabled_statistics,
                nym_api_urls: value.gateway.nym_api_urls,
                nyxd_urls: value.gateway.nyxd_urls,
                statistics_service_url: value.gateway.statistics_service_url,
                cosmos_mnemonic: value.gateway.cosmos_mnemonic,
            },
            // \/ ADDED
            wireguard: Default::default(),
            // /\ ADDED
            storage_paths: GatewayPathsV1_1_31 {
                keys: KeysPathsV1_1_31 {
                    private_identity_key_file: value.storage_paths.keys.private_identity_key_file,
                    public_identity_key_file: value.storage_paths.keys.public_identity_key_file,
                    private_sphinx_key_file: value.storage_paths.keys.private_sphinx_key_file,
                    public_sphinx_key_file: value.storage_paths.keys.public_sphinx_key_file,
                },
                clients_storage: value.storage_paths.clients_storage,
                network_requester_config: value.storage_paths.network_requester_config,
            },
            network_requester: NetworkRequesterV1_1_31 {
                enabled: value.network_requester.enabled,
            },
            logging: LoggingSettingsV1_1_31 {},
            debug: DebugV1_1_31 {
                packet_forwarding_initial_backoff: value.debug.packet_forwarding_initial_backoff,
                packet_forwarding_maximum_backoff: value.debug.packet_forwarding_maximum_backoff,
                initial_connection_timeout: value.debug.initial_connection_timeout,
                maximum_connection_buffer_size: value.debug.maximum_connection_buffer_size,
                presence_sending_delay: value.debug.presence_sending_delay,
                stored_messages_filename_length: value.debug.stored_messages_filename_length,
                message_retrieval_limit: value.debug.message_retrieval_limit,
                use_legacy_framed_packet_version: value.debug.use_legacy_framed_packet_version,
            },
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GatewayV1_1_29 {
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
pub struct NetworkRequesterV1_1_29 {
    pub enabled: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for NetworkRequesterV1_1_29 {
    fn default() -> Self {
        NetworkRequesterV1_1_29 { enabled: false }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct DebugV1_1_29 {
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

impl Default for DebugV1_1_29 {
    fn default() -> Self {
        DebugV1_1_29 {
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
