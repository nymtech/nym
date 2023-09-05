// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::paths::GatewayPaths;
use crate::config::template::CONFIG_TEMPLATE;
use log::debug;
use nym_bin_common::logging::LoggingSettings;
use nym_config::defaults::{DEFAULT_CLIENT_LISTENING_PORT, DEFAULT_MIX_LISTENING_PORT};
use nym_config::helpers::inaddr_any;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_network_defaults::mainnet;
use serde::{Deserialize, Serialize};
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub(crate) mod old_config_v1_1_20;
pub mod persistence;
mod template;

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

/// Derive default path to gateways's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/gateways/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_GATEWAYS_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    pub gateway: Gateway,

    pub storage_paths: GatewayPaths,

    pub network_requester: NetworkRequester,

    #[serde(default)]
    pub logging: LoggingSettings,

    #[serde(default)]
    pub debug: Debug,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Config {
            save_path: None,
            gateway: Gateway::new_default(id.as_ref()),
            storage_paths: GatewayPaths::new_default(id.as_ref()),
            network_requester: Default::default(),
            logging: Default::default(),
            debug: Default::default(),
        }
    }

    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref();
        let mut loaded: Config = read_config_from_toml_file(path)?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::read_from_path(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_path(default_config_filepath(id))
    }

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.gateway.id)
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    pub fn with_enabled_network_requester(mut self, enabled_network_requester: bool) -> Self {
        self.network_requester.enabled = enabled_network_requester;
        self
    }

    pub fn with_default_network_requester_config_path(mut self) -> Self {
        self.storage_paths = self
            .storage_paths
            .with_default_network_requester_config(&self.gateway.id);
        self
    }

    pub fn with_only_coconut_credentials(mut self, only_coconut_credentials: bool) -> Self {
        self.gateway.only_coconut_credentials = only_coconut_credentials;
        self
    }

    pub fn with_enabled_statistics(mut self, enabled_statistics: bool) -> Self {
        self.gateway.enabled_statistics = enabled_statistics;
        self
    }

    pub fn with_custom_statistics_service_url(mut self, statistics_service_url: Url) -> Self {
        self.gateway.statistics_service_url = statistics_service_url;
        self
    }

    pub fn with_custom_nym_apis(mut self, nym_api_urls: Vec<Url>) -> Self {
        self.gateway.nym_api_urls = nym_api_urls;
        self
    }

    pub fn with_custom_validator_nyxd(mut self, validator_nyxd_urls: Vec<Url>) -> Self {
        self.gateway.nyxd_urls = validator_nyxd_urls;
        self
    }

    pub fn with_cosmos_mnemonic(mut self, cosmos_mnemonic: bip39::Mnemonic) -> Self {
        self.gateway.cosmos_mnemonic = cosmos_mnemonic;
        self
    }

    pub fn with_listening_address(mut self, listening_address: IpAddr) -> Self {
        self.gateway.listening_address = listening_address;
        self
    }

    pub fn with_mix_port(mut self, port: u16) -> Self {
        self.gateway.mix_port = port;
        self
    }

    pub fn with_clients_port(mut self, port: u16) -> Self {
        self.gateway.clients_port = port;
        self
    }

    pub fn with_custom_persistent_store(mut self, store_dir: PathBuf) -> Self {
        self.storage_paths.clients_storage = store_dir;
        self
    }

    pub fn get_statistics_service_url(&self) -> Url {
        self.gateway.statistics_service_url.clone()
    }

    pub fn get_nym_api_endpoints(&self) -> Vec<Url> {
        self.gateway.nym_api_urls.clone()
    }

    pub fn get_nyxd_urls(&self) -> Vec<Url> {
        self.gateway.nyxd_urls.clone()
    }

    pub fn get_cosmos_mnemonic(&self) -> bip39::Mnemonic {
        self.gateway.cosmos_mnemonic.clone()
    }
}

// we only really care about the mnemonic being zeroized
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct Gateway {
    /// Version of the gateway for which this configuration was created.
    pub version: String,

    /// ID specifies the human readable ID of this particular gateway.
    pub id: String,

    /// Indicates whether this gateway is accepting only coconut credentials for accessing the
    /// the mixnet, or if it also accepts non-paying clients
    #[serde(default)]
    pub only_coconut_credentials: bool,

    /// Address to which this mixnode will bind to and will be listening for packets.
    #[zeroize(skip)]
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
    #[zeroize(skip)]
    pub statistics_service_url: Url,

    /// Addresses to APIs from which the node gets the view of the network.
    #[serde(alias = "validator_api_urls")]
    #[zeroize(skip)]
    pub nym_api_urls: Vec<Url>,

    /// Addresses to validators which the node uses to check for double spending of ERC20 tokens.
    #[serde(alias = "validator_nymd_urls")]
    #[zeroize(skip)]
    pub nyxd_urls: Vec<Url>,

    /// Mnemonic of a cosmos wallet used in checking for double spending.
    // #[deprecated(note = "move to storage")]
    // TODO: I don't think this should be stored directly in the config...
    pub cosmos_mnemonic: bip39::Mnemonic,
}

impl Gateway {
    pub fn new_default<S: Into<String>>(id: S) -> Self {
        Gateway {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: id.into(),
            only_coconut_credentials: false,
            listening_address: inaddr_any(),
            mix_port: DEFAULT_MIX_LISTENING_PORT,
            clients_port: DEFAULT_CLIENT_LISTENING_PORT,
            enabled_statistics: false,
            statistics_service_url: mainnet::STATISTICS_SERVICE_DOMAIN_ADDRESS
                .parse()
                .expect("Invalid default statistics service URL"),
            nym_api_urls: vec![mainnet::NYM_API.parse().expect("Invalid default API URL")],
            nyxd_urls: vec![mainnet::NYXD_URL.parse().expect("Invalid default nyxd URL")],
            cosmos_mnemonic: bip39::Mnemonic::generate(24).unwrap(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct NetworkRequester {
    /// Specifies whether network requester service is enabled in this process.
    pub enabled: bool,
}

impl Default for NetworkRequester {
    fn default() -> Self {
        NetworkRequester { enabled: false }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Debug {
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

impl Default for Debug {
    fn default() -> Self {
        Debug {
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
