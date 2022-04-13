// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::*;
use config::NymConfig;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

pub mod persistence;

pub const MISSING_VALUE: &str = "MISSING VALUE";

// 'DEBUG'
const DEFAULT_ACK_WAIT_MULTIPLIER: f64 = 1.5;

const DEFAULT_ACK_WAIT_ADDITION: Duration = Duration::from_millis(1_500);
const DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(20);
const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(50);
const DEFAULT_TOPOLOGY_REFRESH_RATE: Duration = Duration::from_secs(5 * 60); // every 5min
const DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT: Duration = Duration::from_millis(5_000);
// Set this to a high value for now, so that we don't risk sporadic timeouts that might cause
// bought bandwidth tokens to not have time to be spent; Once we remove the gateway from the
// bandwidth bridging protocol, we can come back to a smaller timeout value
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub fn missing_string_value() -> String {
    MISSING_VALUE.to_string()
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config<T> {
    client: Client<T>,

    #[serde(default)]
    logging: Logging,
    #[serde(default)]
    debug: Debug,
}

impl<T: NymConfig> Config<T> {
    pub fn new<S: Into<String>>(id: S) -> Self {
        let mut cfg = Config::default();
        cfg.with_id(id);
        cfg
    }

    pub fn with_id<S: Into<String>>(&mut self, id: S) {
        let id = id.into();

        // identity key setting
        if self.client.private_identity_key_file.as_os_str().is_empty() {
            self.client.private_identity_key_file =
                self::Client::<T>::default_private_identity_key_file(&id);
        }
        if self.client.public_identity_key_file.as_os_str().is_empty() {
            self.client.public_identity_key_file =
                self::Client::<T>::default_public_identity_key_file(&id);
        }

        // encryption key setting
        if self
            .client
            .private_encryption_key_file
            .as_os_str()
            .is_empty()
        {
            self.client.private_encryption_key_file =
                self::Client::<T>::default_private_encryption_key_file(&id);
        }
        if self
            .client
            .public_encryption_key_file
            .as_os_str()
            .is_empty()
        {
            self.client.public_encryption_key_file =
                self::Client::<T>::default_public_encryption_key_file(&id);
        }

        // shared gateway key setting
        if self.client.gateway_shared_key_file.as_os_str().is_empty() {
            self.client.gateway_shared_key_file =
                self::Client::<T>::default_gateway_shared_key_file(&id);
        }

        // ack key setting
        if self.client.ack_key_file.as_os_str().is_empty() {
            self.client.ack_key_file = self::Client::<T>::default_ack_key_file(&id);
        }

        if self
            .client
            .reply_encryption_key_store_path
            .as_os_str()
            .is_empty()
        {
            self.client.reply_encryption_key_store_path =
                self::Client::<T>::default_reply_encryption_key_store_path(&id);
        }

        if self.client.database_path.as_os_str().is_empty() {
            self.client.database_path = self::Client::<T>::default_database_path(&id);
        }

        self.client.id = id;
    }

    pub fn with_testnet_mode(&mut self, testnet_mode: bool) {
        self.client.testnet_mode = testnet_mode;
    }

    pub fn with_gateway_endpoint<S: Into<String>>(&mut self, id: S, owner: S, listener: S) {
        self.client.gateway_endpoint = GatewayEndpoint {
            gateway_id: id.into(),
            gateway_owner: owner.into(),
            gateway_listener: listener.into(),
        };
    }

    pub fn with_gateway_id<S: Into<String>>(&mut self, id: S) {
        self.client.gateway_endpoint.gateway_id = id.into();
    }

    #[cfg(not(feature = "coconut"))]
    pub fn with_eth_private_key<S: Into<String>>(&mut self, eth_private_key: S) {
        self.client.eth_private_key = eth_private_key.into();
    }

    #[cfg(not(feature = "coconut"))]
    pub fn with_eth_endpoint<S: Into<String>>(&mut self, eth_endpoint: S) {
        self.client.eth_endpoint = eth_endpoint.into();
    }

    pub fn set_custom_validator_apis(&mut self, validator_api_urls: Vec<Url>) {
        self.client.validator_api_urls = validator_api_urls;
    }

    pub fn set_high_default_traffic_volume(&mut self) {
        self.debug.average_packet_delay = Duration::from_millis(10);
        self.debug.loop_cover_traffic_average_delay = Duration::from_millis(2000000); // basically don't really send cover messages
        self.debug.message_sending_average_delay = Duration::from_millis(4); // 250 "real" messages / s
    }

    pub fn set_custom_version(&mut self, version: &str) {
        self.client.version = version.to_string();
    }

    pub fn get_id(&self) -> String {
        self.client.id.clone()
    }

    pub fn get_testnet_mode(&self) -> bool {
        self.client.testnet_mode
    }

    pub fn get_nym_root_directory(&self) -> PathBuf {
        self.client.nym_root_directory.clone()
    }

    pub fn get_private_identity_key_file(&self) -> PathBuf {
        self.client.private_identity_key_file.clone()
    }

    pub fn get_public_identity_key_file(&self) -> PathBuf {
        self.client.public_identity_key_file.clone()
    }

    pub fn get_private_encryption_key_file(&self) -> PathBuf {
        self.client.private_encryption_key_file.clone()
    }

    pub fn get_public_encryption_key_file(&self) -> PathBuf {
        self.client.public_encryption_key_file.clone()
    }

    pub fn get_gateway_shared_key_file(&self) -> PathBuf {
        self.client.gateway_shared_key_file.clone()
    }

    pub fn get_reply_encryption_key_store_path(&self) -> PathBuf {
        self.client.reply_encryption_key_store_path.clone()
    }

    pub fn get_ack_key_file(&self) -> PathBuf {
        self.client.ack_key_file.clone()
    }

    pub fn get_validator_api_endpoints(&self) -> Vec<Url> {
        self.client.validator_api_urls.clone()
    }

    pub fn get_gateway_id(&self) -> String {
        self.client.gateway_endpoint.gateway_id.clone()
    }

    pub fn get_gateway_owner(&self) -> String {
        self.client.gateway_endpoint.gateway_owner.clone()
    }

    pub fn get_gateway_listener(&self) -> String {
        self.client.gateway_endpoint.gateway_listener.clone()
    }

    pub fn get_database_path(&self) -> PathBuf {
        self.client.database_path.clone()
    }

    #[cfg(not(feature = "coconut"))]
    pub fn get_eth_endpoint(&self) -> String {
        self.client.eth_endpoint.clone()
    }

    #[cfg(not(feature = "coconut"))]
    pub fn get_eth_private_key(&self) -> String {
        self.client.eth_private_key.clone()
    }

    // Debug getters
    pub fn get_average_packet_delay(&self) -> Duration {
        self.debug.average_packet_delay
    }

    pub fn get_average_ack_delay(&self) -> Duration {
        self.debug.average_ack_delay
    }

    pub fn get_ack_wait_multiplier(&self) -> f64 {
        self.debug.ack_wait_multiplier
    }

    pub fn get_ack_wait_addition(&self) -> Duration {
        self.debug.ack_wait_addition
    }

    pub fn get_loop_cover_traffic_average_delay(&self) -> Duration {
        self.debug.loop_cover_traffic_average_delay
    }

    pub fn get_message_sending_average_delay(&self) -> Duration {
        self.debug.message_sending_average_delay
    }

    pub fn get_gateway_response_timeout(&self) -> Duration {
        self.debug.gateway_response_timeout
    }

    pub fn get_topology_refresh_rate(&self) -> Duration {
        self.debug.topology_refresh_rate
    }

    pub fn get_topology_resolution_timeout(&self) -> Duration {
        self.debug.topology_resolution_timeout
    }

    pub fn get_version(&self) -> &str {
        &self.client.version
    }
}

impl<T: NymConfig> Default for Config<T> {
    fn default() -> Self {
        Config {
            client: Client::<T>::default(),
            logging: Default::default(),
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
struct GatewayEndpoint {
    /// gateway_id specifies ID of the gateway to which the client should send messages.
    /// If initially omitted, a random gateway will be chosen from the available topology.
    gateway_id: String,

    /// Address of the gateway owner to which the client should send messages.
    gateway_owner: String,

    /// Address of the gateway listener to which all client requests should be sent.
    gateway_listener: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Client<T> {
    /// Version of the client for which this configuration was created.
    #[serde(default = "missing_string_value")]
    version: String,

    /// ID specifies the human readable ID of this particular client.
    id: String,

    /// Indicates whether this client is running in a testnet mode, thus attempting
    /// to claim bandwidth without presenting bandwidth credentials.
    #[serde(default)]
    testnet_mode: bool,

    /// Addresses to APIs running on validator from which the client gets the view of the network.
    validator_api_urls: Vec<Url>,

    /// Path to file containing private identity key.
    private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    public_identity_key_file: PathBuf,

    /// Path to file containing private encryption key.
    private_encryption_key_file: PathBuf,

    /// Path to file containing public encryption key.
    public_encryption_key_file: PathBuf,

    /// Path to file containing shared key derived with the specified gateway that is used
    /// for all communication with it.
    gateway_shared_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    ack_key_file: PathBuf,

    /// Full path to file containing reply encryption keys of all reply-SURBs we have ever
    /// sent but not received back.
    reply_encryption_key_store_path: PathBuf,

    /// Information regarding how the client should send data to gateway.
    gateway_endpoint: GatewayEndpoint,

    /// Path to the database containing bandwidth credentials of this client.
    database_path: PathBuf,

    /// Ethereum private key.
    #[cfg(not(feature = "coconut"))]
    eth_private_key: String,

    /// Address to an Ethereum full node.
    #[cfg(not(feature = "coconut"))]
    eth_endpoint: String,

    /// nym_home_directory specifies absolute path to the home nym Clients directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,

    #[serde(skip)]
    super_struct: PhantomData<*const T>,
}

impl<T: NymConfig> Default for Client<T> {
    fn default() -> Self {
        // there must be explicit checks for whether id is not empty later
        Client {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: "".to_string(),
            testnet_mode: false,
            validator_api_urls: default_api_endpoints(),
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
            private_encryption_key_file: Default::default(),
            public_encryption_key_file: Default::default(),
            gateway_shared_key_file: Default::default(),
            ack_key_file: Default::default(),
            reply_encryption_key_store_path: Default::default(),
            gateway_endpoint: Default::default(),
            database_path: Default::default(),
            #[cfg(not(feature = "coconut"))]
            eth_private_key: "".to_string(),
            #[cfg(not(feature = "coconut"))]
            eth_endpoint: "".to_string(),
            nym_root_directory: T::default_root_directory(),
            super_struct: Default::default(),
        }
    }
}

impl<T: NymConfig> Client<T> {
    fn default_private_identity_key_file(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("private_identity.pem")
    }

    fn default_public_identity_key_file(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("public_identity.pem")
    }

    fn default_private_encryption_key_file(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("private_encryption.pem")
    }

    fn default_public_encryption_key_file(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("public_encryption.pem")
    }

    fn default_gateway_shared_key_file(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("gateway_shared.pem")
    }

    fn default_ack_key_file(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("ack_key.pem")
    }

    fn default_reply_encryption_key_store_path(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("reply_key_store")
    }
    fn default_database_path(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("db.sqlite")
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Logging {}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Debug {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent packet is going to be delayed at any given mix node.
    /// So for a packet going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[serde(with = "humantime_serde")]
    average_packet_delay: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// sent acknowledgement is going to be delayed at any given mix node.
    /// So for an ack going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[serde(with = "humantime_serde")]
    average_ack_delay: Duration,

    /// Value multiplied with the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 1.
    ack_wait_multiplier: f64,

    /// Value added to the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 0.
    #[serde(with = "humantime_serde")]
    ack_wait_addition: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    #[serde(with = "humantime_serde")]
    loop_cover_traffic_average_delay: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    #[serde(with = "humantime_serde")]
    message_sending_average_delay: Duration,

    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    #[serde(with = "humantime_serde")]
    gateway_response_timeout: Duration,

    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    #[serde(with = "humantime_serde")]
    topology_refresh_rate: Duration,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    #[serde(with = "humantime_serde")]
    topology_resolution_timeout: Duration,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            average_packet_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            average_ack_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            ack_wait_multiplier: DEFAULT_ACK_WAIT_MULTIPLIER,
            ack_wait_addition: DEFAULT_ACK_WAIT_ADDITION,
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            message_sending_average_delay: DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY,
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
        }
    }
}
