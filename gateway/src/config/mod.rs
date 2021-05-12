// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::config_template;
use config::{deserialize_duration, deserialize_validators, NymConfig};
use log::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

pub mod persistence;
mod template;

pub(crate) const MISSING_VALUE: &str = "MISSING VALUE";

// 'GATEWAY'
const DEFAULT_MIX_LISTENING_PORT: u16 = 1789;
const DEFAULT_CLIENT_LISTENING_PORT: u16 = 9000;
pub(crate) const DEFAULT_VALIDATOR_REST_ENDPOINTS: &[&str] = &[
    "http://testnet-finney-validator.nymtech.net:1317",
    "http://testnet-finney-validator2.nymtech.net:1317",
    "http://mixnet.club:1317",
];
pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = "hal1k0jntykt7e4g3y88ltc60czgjuqdy4c9c6gv94";

// 'DEBUG'
// where applicable, the below are defined in milliseconds
const DEFAULT_PRESENCE_SENDING_DELAY: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_millis(300_000);
const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
const DEFAULT_CACHE_ENTRY_TTL: Duration = Duration::from_millis(30_000);
const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 128;

const DEFAULT_STORED_MESSAGE_FILENAME_LENGTH: u16 = 16;
const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: u16 = 5;

// helper function to get default validators as a Vec<String>
pub fn default_validator_rest_endpoints() -> Vec<String> {
    DEFAULT_VALIDATOR_REST_ENDPOINTS
        .iter()
        .map(|&endpoint| endpoint.to_string())
        .collect()
}

pub fn missing_string_value() -> String {
    MISSING_VALUE.to_string()
}

pub fn missing_vec_string_value() -> Vec<String> {
    vec![missing_string_value()]
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    gateway: Gateway,

    mixnet_endpoint: MixnetEndpoint,

    clients_endpoint: ClientsEndpoint,

    #[serde(default)]
    logging: Logging,
    #[serde(default)]
    debug: Debug,
}

impl NymConfig for Config {
    fn template() -> &'static str {
        config_template()
    }

    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("gateways")
    }

    fn root_directory(&self) -> PathBuf {
        self.gateway.nym_root_directory.clone()
    }

    fn config_directory(&self) -> PathBuf {
        self.gateway
            .nym_root_directory
            .join(&self.gateway.id)
            .join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.gateway
            .nym_root_directory
            .join(&self.gateway.id)
            .join("data")
    }
}

impl Config {
    pub fn new<S: Into<String>>(id: S) -> Self {
        Config::default().with_id(id)
    }

    // builder methods
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        let id = id.into();
        if self.gateway.private_sphinx_key_file.as_os_str().is_empty() {
            self.gateway.private_sphinx_key_file =
                self::Gateway::default_private_sphinx_key_file(&id);
        }
        if self.gateway.public_sphinx_key_file.as_os_str().is_empty() {
            self.gateway.public_sphinx_key_file =
                self::Gateway::default_public_sphinx_key_file(&id);
        }
        if self
            .gateway
            .private_identity_key_file
            .as_os_str()
            .is_empty()
        {
            self.gateway.private_identity_key_file =
                self::Gateway::default_private_identity_key_file(&id);
        }
        if self.gateway.public_identity_key_file.as_os_str().is_empty() {
            self.gateway.public_identity_key_file =
                self::Gateway::default_public_identity_key_file(&id);
        }
        if self
            .clients_endpoint
            .inboxes_directory
            .as_os_str()
            .is_empty()
        {
            self.clients_endpoint.inboxes_directory =
                self::ClientsEndpoint::default_inboxes_directory(&id);
        }
        if self.clients_endpoint.ledger_path.as_os_str().is_empty() {
            self.clients_endpoint.ledger_path = self::ClientsEndpoint::default_ledger_path(&id);
        }

        self.gateway.id = id;
        self
    }

    pub fn with_custom_validators(mut self, validators: Vec<String>) -> Self {
        self.gateway.validator_rest_urls = validators;
        self
    }

    pub fn with_custom_mixnet_contract<S: Into<String>>(mut self, mixnet_contract: S) -> Self {
        self.gateway.mixnet_contract_address = mixnet_contract.into();
        self
    }

    pub fn with_mix_listening_host<S: Into<String>>(mut self, host: S) -> Self {
        // see if the provided `host` is just an ip address or ip:port
        let host = host.into();

        // is it ip:port?
        match SocketAddr::from_str(host.as_ref()) {
            Ok(socket_addr) => {
                self.mixnet_endpoint.listening_address = socket_addr;
                self
            }
            // try just for ip
            Err(_) => match IpAddr::from_str(host.as_ref()) {
                Ok(ip_addr) => {
                    self.mixnet_endpoint.listening_address.set_ip(ip_addr);
                    self
                }
                Err(_) => {
                    error!(
                        "failed to make any changes to config - invalid host {}",
                        host
                    );
                    self
                }
            },
        }
    }

    pub fn with_mix_listening_port(mut self, port: u16) -> Self {
        self.mixnet_endpoint.listening_address.set_port(port);
        self
    }

    pub fn with_mix_announce_host<S: Into<String>>(mut self, host: S) -> Self {
        // this is slightly more complicated as we store announce information as String,
        // since it might not necessarily be a valid SocketAddr (say `nymtech.net:8080` is a valid
        // announce address, yet invalid SocketAddr`

        // first let's see if we received host:port or just host part of an address
        let host = host.into();
        match host.split(':').count() {
            1 => {
                // we provided only 'host' part so we are going to reuse existing port
                self.mixnet_endpoint.announce_address =
                    format!("{}:{}", host, self.mixnet_endpoint.listening_address.port());
                self
            }
            2 => {
                // we provided 'host:port' so just put the whole thing there
                self.mixnet_endpoint.announce_address = host;
                self
            }
            _ => {
                // we provided something completely invalid, so don't try to parse it
                error!(
                    "failed to make any changes to config - invalid announce host {}",
                    host
                );
                self
            }
        }
    }

    pub fn mix_announce_host_from_listening_host(mut self) -> Self {
        self.mixnet_endpoint.announce_address = self.mixnet_endpoint.listening_address.to_string();
        self
    }

    pub fn with_mix_announce_port(mut self, port: u16) -> Self {
        let current_host: Vec<_> = self.mixnet_endpoint.announce_address.split(':').collect();
        debug_assert_eq!(current_host.len(), 2);
        self.mixnet_endpoint.announce_address = format!("{}:{}", current_host[0], port);
        self
    }

    pub fn with_clients_listening_host<S: Into<String>>(mut self, host: S) -> Self {
        // see if the provided `host` is just an ip address or ip:port
        let host = host.into();

        // is it ip:port?
        match SocketAddr::from_str(host.as_ref()) {
            Ok(socket_addr) => {
                self.clients_endpoint.listening_address = socket_addr;
                self
            }
            // try just for ip
            Err(_) => match IpAddr::from_str(host.as_ref()) {
                Ok(ip_addr) => {
                    self.clients_endpoint.listening_address.set_ip(ip_addr);
                    self
                }
                Err(_) => {
                    error!(
                        "failed to make any changes to config - invalid host {}",
                        host
                    );
                    self
                }
            },
        }
    }

    pub fn clients_announce_host_from_listening_host(mut self) -> Self {
        self.clients_endpoint.announce_address = format!(
            "ws://{}",
            self.clients_endpoint.listening_address.to_string()
        );
        self
    }

    pub fn with_clients_listening_port(mut self, port: u16) -> Self {
        self.clients_endpoint.listening_address.set_port(port);
        self
    }

    pub fn with_clients_announce_host<S: Into<String>>(mut self, host: S) -> Self {
        // this is slightly more complicated as we store announce information as String,
        // since it might not necessarily be a valid SocketAddr (say `nymtech.net:8080` is a valid
        // announce address, yet invalid SocketAddr`

        // first let's see if we received host:port or just host part of an address
        let host = host.into();
        match host.split(':').count() {
            1 => {
                // we provided only 'host' part so we are going to reuse existing port
                self.clients_endpoint.announce_address = format!(
                    "{}:{}",
                    host,
                    self.clients_endpoint.listening_address.port()
                );
                // make sure it has 'ws' prefix (by extension it also includes 'wss')
                if !self.clients_endpoint.announce_address.starts_with("ws") {
                    self.clients_endpoint.announce_address =
                        format!("ws://{}", self.clients_endpoint.announce_address);
                }
                self
            }
            2 => {
                // we provided 'host:port' so just put the whole thing there
                self.clients_endpoint.announce_address = host;
                // make sure it has 'ws' prefix (by extension it also includes 'wss')
                if !self.clients_endpoint.announce_address.starts_with("ws") {
                    self.clients_endpoint.announce_address =
                        format!("ws://{}", self.clients_endpoint.announce_address);
                }
                self
            }
            _ => {
                // we provided something completely invalid, so don't try to parse it
                error!(
                    "failed to make any changes to config - invalid announce host {}",
                    host
                );
                self
            }
        }
    }

    pub fn with_clients_announce_port(mut self, port: u16) -> Self {
        let current_host: Vec<_> = self.clients_endpoint.announce_address.split(':').collect();
        debug_assert_eq!(current_host.len(), 2);
        self.clients_endpoint.announce_address = format!("{}:{}", current_host[0], port);
        self
    }

    pub fn with_custom_clients_inboxes<S: Into<String>>(mut self, inboxes_dir: S) -> Self {
        self.clients_endpoint.inboxes_directory = PathBuf::from(inboxes_dir.into());
        self
    }

    pub fn with_custom_clients_ledger<S: Into<String>>(mut self, ledger_path: S) -> Self {
        self.clients_endpoint.ledger_path = PathBuf::from(ledger_path.into());
        self
    }

    pub fn with_custom_version(mut self, version: &str) -> Self {
        self.gateway.version = version.to_string();
        self
    }

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }

    pub fn get_private_identity_key_file(&self) -> PathBuf {
        self.gateway.private_identity_key_file.clone()
    }

    pub fn get_public_identity_key_file(&self) -> PathBuf {
        self.gateway.public_identity_key_file.clone()
    }

    pub fn get_private_sphinx_key_file(&self) -> PathBuf {
        self.gateway.private_sphinx_key_file.clone()
    }

    pub fn get_public_sphinx_key_file(&self) -> PathBuf {
        self.gateway.public_sphinx_key_file.clone()
    }

    pub fn get_validator_rest_endpoints(&self) -> Vec<String> {
        self.gateway.validator_rest_urls.clone()
    }

    pub fn get_validator_mixnet_contract_address(&self) -> String {
        self.gateway.mixnet_contract_address.clone()
    }

    pub fn get_mix_listening_address(&self) -> SocketAddr {
        self.mixnet_endpoint.listening_address
    }

    pub fn get_mix_announce_address(&self) -> String {
        self.mixnet_endpoint.announce_address.clone()
    }

    pub fn get_clients_listening_address(&self) -> SocketAddr {
        self.clients_endpoint.listening_address
    }

    pub fn get_clients_announce_address(&self) -> String {
        self.clients_endpoint.announce_address.clone()
    }

    pub fn get_clients_inboxes_dir(&self) -> PathBuf {
        self.clients_endpoint.inboxes_directory.clone()
    }

    pub fn get_clients_ledger_path(&self) -> PathBuf {
        self.clients_endpoint.ledger_path.clone()
    }

    pub fn get_packet_forwarding_initial_backoff(&self) -> Duration {
        self.debug.packet_forwarding_initial_backoff
    }

    pub fn get_packet_forwarding_maximum_backoff(&self) -> Duration {
        self.debug.packet_forwarding_maximum_backoff
    }

    pub fn get_initial_connection_timeout(&self) -> Duration {
        self.debug.initial_connection_timeout
    }

    pub fn get_maximum_connection_buffer_size(&self) -> usize {
        self.debug.maximum_connection_buffer_size
    }

    pub fn get_message_retrieval_limit(&self) -> u16 {
        self.debug.message_retrieval_limit
    }

    pub fn get_stored_messages_filename_length(&self) -> u16 {
        self.debug.stored_messages_filename_length
    }

    pub fn get_cache_entry_ttl(&self) -> Duration {
        self.debug.cache_entry_ttl
    }

    pub fn get_version(&self) -> &str {
        &self.gateway.version
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Gateway {
    /// Version of the gateway for which this configuration was created.
    #[serde(default = "missing_string_value")]
    version: String,

    /// ID specifies the human readable ID of this particular gateway.
    id: String,

    /// Path to file containing private identity key.
    private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    public_identity_key_file: PathBuf,

    /// Path to file containing private sphinx key.
    private_sphinx_key_file: PathBuf,

    /// Path to file containing public sphinx key.
    public_sphinx_key_file: PathBuf,

    /// Validator server to which the node will be reporting their presence data.
    #[serde(
        deserialize_with = "deserialize_validators",
        default = "missing_vec_string_value",
        alias = "validator_rest_url"
    )]
    validator_rest_urls: Vec<String>,

    /// Address of the validator contract managing the network.
    #[serde(default = "missing_string_value")]
    mixnet_contract_address: String,

    /// nym_home_directory specifies absolute path to the home nym gateways directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,
}

impl Gateway {
    fn default_private_sphinx_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("private_sphinx.pem")
    }

    fn default_public_sphinx_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("public_sphinx.pem")
    }

    fn default_private_identity_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("private_identity.pem")
    }

    fn default_public_identity_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("public_identity.pem")
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Gateway {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: "".to_string(),
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
            private_sphinx_key_file: Default::default(),
            public_sphinx_key_file: Default::default(),
            validator_rest_urls: default_validator_rest_endpoints(),
            mixnet_contract_address: DEFAULT_MIXNET_CONTRACT_ADDRESS.to_string(),
            nym_root_directory: Config::default_root_directory(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixnetEndpoint {
    /// Socket address to which this gateway will bind to
    /// and will be listening for sphinx packets coming from the mixnet.
    listening_address: SocketAddr,

    /// Optional address announced to the directory server for the clients to connect to.
    /// It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
    /// later on by using name resolvable with a DNS query, such as `nymtech.net:8080`.
    /// Additionally a custom port can be provided, so both `nymtech.net:8080` and `nymtech.net`
    /// are valid announce addresses, while the later will default to whatever port is used for
    /// `listening_address`.
    announce_address: String,
}

impl Default for MixnetEndpoint {
    fn default() -> Self {
        MixnetEndpoint {
            listening_address: format!("0.0.0.0:{}", DEFAULT_MIX_LISTENING_PORT)
                .parse()
                .unwrap(),
            announce_address: format!("127.0.0.1:{}", DEFAULT_MIX_LISTENING_PORT),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClientsEndpoint {
    /// Socket address to which this gateway will bind to
    /// and will be listening for data packets coming from the clients.
    listening_address: SocketAddr,

    /// Optional address announced to the directory server for the clients to connect to.
    /// It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
    /// later on by using name resolvable with a DNS query, such as `nymtech.net:8080`.
    /// Additionally a custom port can be provided, so both `nymtech.net:8080` and `nymtech.net`
    /// are valid announce addresses, while the later will default to whatever port is used for
    /// `listening_address`.
    announce_address: String,

    /// Path to the directory with clients inboxes containing messages stored for them.
    inboxes_directory: PathBuf,

    /// Full path to a file containing mapping of
    /// client addresses to their access tokens.
    ledger_path: PathBuf,
}

impl ClientsEndpoint {
    fn default_inboxes_directory(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("inboxes")
    }

    fn default_ledger_path(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("client_ledger.sled")
    }
}

impl Default for ClientsEndpoint {
    fn default() -> Self {
        ClientsEndpoint {
            listening_address: format!("0.0.0.0:{}", DEFAULT_CLIENT_LISTENING_PORT)
                .parse()
                .unwrap(),
            announce_address: format!("ws://127.0.0.1:{}", DEFAULT_CLIENT_LISTENING_PORT),
            inboxes_directory: Default::default(),
            ledger_path: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Logging {}

impl Default for Logging {
    fn default() -> Self {
        Logging {}
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Debug {
    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    packet_forwarding_initial_backoff: Duration,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    packet_forwarding_maximum_backoff: Duration,

    /// Timeout for establishing initial connection when trying to forward a sphinx packet.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    initial_connection_timeout: Duration,

    /// Maximum number of packets that can be stored waiting to get sent to a particular connection.
    maximum_connection_buffer_size: usize,

    /// Delay between each subsequent presence data being sent.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    presence_sending_delay: Duration,

    /// Length of filenames for new client messages.
    stored_messages_filename_length: u16,

    /// Number of messages client gets on each request
    /// if there are no real messages, dummy ones are created to always return  
    /// `message_retrieval_limit` total messages
    message_retrieval_limit: u16,

    /// Duration for which a cached vpn processing result is going to get stored for.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    cache_entry_ttl: Duration,
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
            cache_entry_ttl: DEFAULT_CACHE_ENTRY_TTL,
        }
    }
}
