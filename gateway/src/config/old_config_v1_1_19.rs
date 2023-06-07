// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::paths::{GatewayPaths, KeysPaths};
use crate::config::{Config, Debug, Gateway};
use nym_bin_common::logging::LoggingSettings;
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use nym_validator_client::nyxd;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

const STATISTICS_SERVICE_DOMAIN_ADDRESS: &str = "https://mainnet-stats.nymte.ch:8090/";
const NYXD_URL: &str = "https://rpc.nymtech.net";
const NYM_API: &str = "https://validator.nymtech.net/api/";
const DEFAULT_MIX_LISTENING_PORT: u16 = 1789;
const DEFAULT_CLIENT_LISTENING_PORT: u16 = 9000;

const DEFAULT_PRESENCE_SENDING_DELAY: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_millis(300_000);
const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 2000;

const DEFAULT_STORED_MESSAGE_FILENAME_LENGTH: u16 = 16;
const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;

fn bind_all_address() -> IpAddr {
    "0.0.0.0".parse().unwrap()
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ConfigV1_1_19 {
    gateway: GatewayV1_1_19,

    #[serde(default)]
    logging: LoggingV1_1_19,
    #[serde(default)]
    debug: DebugV1_1_19,
}

impl From<ConfigV1_1_19> for Config {
    fn from(value: ConfigV1_1_19) -> Self {
        Config {
            gateway: Gateway {
                version: value.gateway.version,
                id: value.gateway.id,
                only_coconut_credentials: value.gateway.only_coconut_credentials,
                listening_address: value.gateway.listening_address,
                mix_port: value.gateway.mix_port,
                clients_port: value.gateway.clients_port,
                enabled_statistics: value.gateway.enabled_statistics,
                nym_api_urls: value.gateway.nym_api_urls,
                nyxd_urls: value.gateway.nyxd_urls,
                statistics_service_url: value.gateway.statistics_service_url,
                cosmos_mnemonic: value.gateway.cosmos_mnemonic,
            },
            storage_paths: GatewayPaths {
                keys: KeysPaths {
                    private_identity_key_file: value.gateway.private_identity_key_file,
                    public_identity_key_file: value.gateway.public_identity_key_file,
                    private_sphinx_key_file: value.gateway.private_sphinx_key_file,
                    public_sphinx_key_file: value.gateway.public_sphinx_key_file,
                },
                clients_storage: value.gateway.persistent_storage,
            },
            logging: value.logging.into(),
            debug: value.debug.into(),
        }
    }
}

impl MigrationNymConfig for ConfigV1_1_19 {
    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("gateways")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GatewayV1_1_19 {
    version: String,
    id: String,

    #[serde(default)]
    only_coconut_credentials: bool,
    #[serde(default = "bind_all_address")]
    listening_address: IpAddr,
    announce_address: String,
    mix_port: u16,
    clients_port: u16,
    private_identity_key_file: PathBuf,
    public_identity_key_file: PathBuf,
    private_sphinx_key_file: PathBuf,
    public_sphinx_key_file: PathBuf,
    enabled_statistics: bool,
    statistics_service_url: Url,
    #[serde(alias = "validator_api_urls")]
    nym_api_urls: Vec<Url>,
    #[serde(alias = "validator_nymd_urls")]
    nyxd_urls: Vec<Url>,
    cosmos_mnemonic: bip39::Mnemonic,
    nym_root_directory: PathBuf,
    persistent_storage: PathBuf,
    wallet_address: Option<nyxd::AccountId>,
}

impl Default for GatewayV1_1_19 {
    fn default() -> Self {
        GatewayV1_1_19 {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: "".to_string(),
            only_coconut_credentials: false,
            listening_address: bind_all_address(),
            announce_address: "127.0.0.1".to_string(),
            mix_port: DEFAULT_MIX_LISTENING_PORT,
            clients_port: DEFAULT_CLIENT_LISTENING_PORT,
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
            private_sphinx_key_file: Default::default(),
            public_sphinx_key_file: Default::default(),
            enabled_statistics: false,
            statistics_service_url: Url::from_str(STATISTICS_SERVICE_DOMAIN_ADDRESS)
                .expect("Invalid default statistics service URL"),
            nym_api_urls: vec![Url::from_str(NYM_API).expect("Invalid default API URL")],
            nyxd_urls: vec![Url::from_str(NYXD_URL).expect("Invalid default nyxd URL")],
            cosmos_mnemonic: bip39::Mnemonic::generate(24).unwrap(),
            nym_root_directory: ConfigV1_1_19::default_root_directory(),
            persistent_storage: Default::default(),
            wallet_address: None,
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct LoggingV1_1_19 {}

impl From<LoggingV1_1_19> for LoggingSettings {
    fn from(_value: LoggingV1_1_19) -> Self {
        LoggingSettings {}
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
struct DebugV1_1_19 {
    #[serde(with = "humantime_serde")]
    packet_forwarding_initial_backoff: Duration,
    #[serde(with = "humantime_serde")]
    packet_forwarding_maximum_backoff: Duration,
    #[serde(with = "humantime_serde")]
    initial_connection_timeout: Duration,
    maximum_connection_buffer_size: usize,
    #[serde(with = "humantime_serde")]
    presence_sending_delay: Duration,
    stored_messages_filename_length: u16,
    message_retrieval_limit: i64,
    use_legacy_framed_packet_version: bool,
}

impl From<DebugV1_1_19> for Debug {
    fn from(value: DebugV1_1_19) -> Self {
        Debug {
            packet_forwarding_initial_backoff: value.packet_forwarding_initial_backoff,
            packet_forwarding_maximum_backoff: value.packet_forwarding_maximum_backoff,
            initial_connection_timeout: value.initial_connection_timeout,
            maximum_connection_buffer_size: value.maximum_connection_buffer_size,
            presence_sending_delay: value.presence_sending_delay,
            stored_messages_filename_length: value.stored_messages_filename_length,
            message_retrieval_limit: value.message_retrieval_limit,
            use_legacy_framed_packet_version: value.use_legacy_framed_packet_version,
        }
    }
}

impl Default for DebugV1_1_19 {
    fn default() -> Self {
        DebugV1_1_19 {
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            presence_sending_delay: DEFAULT_PRESENCE_SENDING_DELAY,
            maximum_connection_buffer_size: DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            stored_messages_filename_length: DEFAULT_STORED_MESSAGE_FILENAME_LENGTH,
            message_retrieval_limit: DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
            // TODO: remember to change it in one of future releases!!
            use_legacy_framed_packet_version: true,
        }
    }
}
