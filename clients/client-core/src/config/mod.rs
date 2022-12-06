// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::NymConfig;
use nymsphinx::params::PacketSize;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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

// reply-surbs related:

// define when to request
// clients/client-core/src/client/replies/reply_storage/surb_storage.rs
const DEFAULT_MINIMUM_REPLY_SURB_STORAGE_THRESHOLD: usize = 10;
const DEFAULT_MAXIMUM_REPLY_SURB_STORAGE_THRESHOLD: usize = 200;

// define how much to request at once
// clients/client-core/src/client/replies/reply_controller.rs
const DEFAULT_MINIMUM_REPLY_SURB_REQUEST_SIZE: u32 = 10;
const DEFAULT_MAXIMUM_REPLY_SURB_REQUEST_SIZE: u32 = 100;

const DEFAULT_MAXIMUM_ALLOWED_SURB_REQUEST_SIZE: u32 = 500;

const DEFAULT_RETRANSMISSION_REPLY_SURB_REQUEST_SIZE: u32 = 10;
const DEFAULT_MAXIMUM_REPLY_SURB_WAITING_PERIOD: Duration = Duration::from_secs(10);

// 24 hours
const DEFAULT_MAXIMUM_REPLY_SURB_AGE: Duration = Duration::from_secs(24 * 60 * 60);

pub fn missing_string_value() -> String {
    MISSING_VALUE.to_string()
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config<T> {
    client: Client<T>,

    #[serde(default)]
    logging: Logging,
    #[serde(default)]
    debug: DebugConfig,
}

impl<T> Config<T> {
    pub fn new<S: Into<String>>(id: S) -> Self
    where
        T: NymConfig,
    {
        Config::default().with_id(id)
    }

    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self
    where
        T: NymConfig,
    {
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

        if self.client.reply_surb_database_path.as_os_str().is_empty() {
            self.client.reply_surb_database_path =
                self::Client::<T>::default_reply_surb_database_path(&id);
        }

        if self.client.database_path.as_os_str().is_empty() {
            self.client.database_path = self::Client::<T>::default_database_path(&id);
        }

        self.client.id = id;
        self
    }

    pub fn with_disabled_credentials(&mut self, disabled_credentials_mode: bool) {
        self.client.disabled_credentials_mode = disabled_credentials_mode;
    }

    pub fn with_gateway_endpoint(&mut self, gateway_endpoint: GatewayEndpointConfig) {
        self.client.gateway_endpoint = gateway_endpoint;
    }

    pub fn with_gateway_id<S: Into<String>>(&mut self, id: S) {
        self.client.gateway_endpoint.gateway_id = id.into();
    }

    pub fn set_custom_validators(&mut self, validator_urls: Vec<Url>) {
        self.client.validator_urls = validator_urls;
    }

    pub fn set_custom_validator_apis(&mut self, validator_api_urls: Vec<Url>) {
        self.client.validator_api_urls = validator_api_urls;
    }

    pub fn set_high_default_traffic_volume(&mut self) {
        self.debug.average_packet_delay = Duration::from_millis(10);
        // basically don't really send cover messages
        self.debug.loop_cover_traffic_average_delay = Duration::from_millis(2_000_000);
        // 250 "real" messages / s
        self.debug.message_sending_average_delay = Duration::from_millis(4);
    }

    pub fn set_no_cover_traffic(&mut self) {
        self.debug.disable_loop_cover_traffic_stream = true;
        self.debug.disable_main_poisson_packet_distribution = true;
    }

    pub fn set_custom_version(&mut self, version: &str) {
        self.client.version = version.to_string();
    }

    pub fn get_id(&self) -> String {
        self.client.id.clone()
    }

    pub fn get_disabled_credentials_mode(&self) -> bool {
        self.client.disabled_credentials_mode
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

    pub fn get_ack_key_file(&self) -> PathBuf {
        self.client.ack_key_file.clone()
    }

    pub fn get_validator_endpoints(&self) -> Vec<Url> {
        self.client.validator_urls.clone()
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

    pub fn get_gateway_endpoint_config(&self) -> &GatewayEndpointConfig {
        &self.client.gateway_endpoint
    }

    pub fn get_gateway_endpoint(&self) -> &GatewayEndpointConfig {
        &self.client.gateway_endpoint
    }

    pub fn get_database_path(&self) -> PathBuf {
        self.client.database_path.clone()
    }

    pub fn get_reply_surb_database_path(&self) -> PathBuf {
        self.client.reply_surb_database_path.clone()
    }

    pub fn get_version(&self) -> &str {
        &self.client.version
    }

    // Debug getters
    pub fn get_debug_config(&self) -> &DebugConfig {
        &self.debug
    }

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

    pub fn get_disabled_loop_cover_traffic_stream(&self) -> bool {
        self.debug.disable_loop_cover_traffic_stream
    }

    pub fn get_disabled_main_poisson_packet_distribution(&self) -> bool {
        self.debug.disable_main_poisson_packet_distribution
    }

    pub fn get_use_extended_packet_size(&self) -> Option<ExtendedPacketSize> {
        self.debug.use_extended_packet_size
    }

    pub fn get_minimum_reply_surb_storage_threshold(&self) -> usize {
        self.debug.minimum_reply_surb_storage_threshold
    }

    pub fn get_maximum_reply_surb_storage_threshold(&self) -> usize {
        self.debug.maximum_reply_surb_storage_threshold
    }

    pub fn get_minimum_reply_surb_request_size(&self) -> u32 {
        self.debug.minimum_reply_surb_request_size
    }

    pub fn get_maximum_reply_surb_request_size(&self) -> u32 {
        self.debug.maximum_reply_surb_request_size
    }

    pub fn get_maximum_allowed_reply_surb_request_size(&self) -> u32 {
        self.debug.maximum_allowed_reply_surb_request_size
    }

    pub fn get_retransmission_reply_surb_request_size(&self) -> u32 {
        self.debug.retransmission_reply_surb_request_size
    }

    pub fn get_maximum_reply_surb_waiting_period(&self) -> Duration {
        self.debug.maximum_reply_surb_waiting_period
    }

    pub fn get_maximum_reply_surb_age(&self) -> Duration {
        self.debug.maximum_reply_surb_age
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

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
pub struct GatewayEndpointConfig {
    /// gateway_id specifies ID of the gateway to which the client should send messages.
    /// If initially omitted, a random gateway will be chosen from the available topology.
    pub gateway_id: String,

    /// Address of the gateway owner to which the client should send messages.
    pub gateway_owner: String,

    /// Address of the gateway listener to which all client requests should be sent.
    pub gateway_listener: String,
}

impl From<topology::gateway::Node> for GatewayEndpointConfig {
    fn from(node: topology::gateway::Node) -> GatewayEndpointConfig {
        let gateway_listener = node.clients_address();
        GatewayEndpointConfig {
            gateway_id: node.identity_key.to_base58_string(),
            gateway_owner: node.owner,
            gateway_listener,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct Client<T> {
    /// Version of the client for which this configuration was created.
    #[serde(default = "missing_string_value")]
    version: String,

    /// ID specifies the human readable ID of this particular client.
    id: String,

    /// Indicates whether this client is running in a disabled credentials mode, thus attempting
    /// to claim bandwidth without presenting bandwidth credentials.
    #[serde(default)]
    disabled_credentials_mode: bool,

    /// Addresses to nymd validators via which the client can communicate with the chain.
    #[serde(default)]
    validator_urls: Vec<Url>,

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

    /// Information regarding how the client should send data to gateway.
    gateway_endpoint: GatewayEndpointConfig,

    /// Path to the database containing bandwidth credentials of this client.
    database_path: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    reply_surb_database_path: PathBuf,

    /// nym_home_directory specifies absolute path to the home nym Clients directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,

    #[serde(skip)]
    super_struct: PhantomData<T>,
}

impl<T: NymConfig> Default for Client<T> {
    fn default() -> Self {
        // there must be explicit checks for whether id is not empty later
        Client {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: "".to_string(),
            disabled_credentials_mode: true,
            validator_urls: vec![],
            validator_api_urls: vec![],
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
            private_encryption_key_file: Default::default(),
            public_encryption_key_file: Default::default(),
            gateway_shared_key_file: Default::default(),
            ack_key_file: Default::default(),
            gateway_endpoint: Default::default(),
            database_path: Default::default(),
            reply_surb_database_path: Default::default(),
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

    fn default_reply_surb_database_path(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("persistent_reply_store.sqlite")
    }

    fn default_database_path(id: &str) -> PathBuf {
        T::default_data_directory(Some(id)).join("db.sqlite")
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Logging {}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugConfig {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent packet is going to be delayed at any given mix node.
    /// So for a packet going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[serde(with = "humantime_serde")]
    pub average_packet_delay: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// sent acknowledgement is going to be delayed at any given mix node.
    /// So for an ack going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[serde(with = "humantime_serde")]
    pub average_ack_delay: Duration,

    /// Value multiplied with the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 1.
    pub ack_wait_multiplier: f64,

    /// Value added to the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 0.
    #[serde(with = "humantime_serde")]
    pub ack_wait_addition: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    #[serde(with = "humantime_serde")]
    pub loop_cover_traffic_average_delay: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    #[serde(with = "humantime_serde")]
    pub message_sending_average_delay: Duration,

    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    #[serde(with = "humantime_serde")]
    pub gateway_response_timeout: Duration,

    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    #[serde(with = "humantime_serde")]
    pub topology_refresh_rate: Duration,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    #[serde(with = "humantime_serde")]
    pub topology_resolution_timeout: Duration,

    /// Controls whether the dedicated loop cover traffic stream should be enabled.
    /// (and sending packets, on average, every [Self::loop_cover_traffic_average_delay])
    pub disable_loop_cover_traffic_stream: bool,

    /// Controls whether the main packet stream constantly produces packets according to the predefined
    /// poisson distribution.
    pub disable_main_poisson_packet_distribution: bool,

    /// Controls whether the sent sphinx packet use a NON-DEFAULT bigger size.
    pub use_extended_packet_size: Option<ExtendedPacketSize>,

    /// Defines the minimum number of reply surbs the client wants to keep in its storage at all times.
    /// It can only allow to go below that value if its to request additional reply surbs.
    pub minimum_reply_surb_storage_threshold: usize,

    /// Defines the maximum number of reply surbs the client wants to keep in its storage at any times.
    pub maximum_reply_surb_storage_threshold: usize,

    /// Defines the minimum number of reply surbs the client would request.
    pub minimum_reply_surb_request_size: u32,

    /// Defines the maximum number of reply surbs the client would request.
    pub maximum_reply_surb_request_size: u32,

    /// Defines the maximum number of reply surbs a remote party is allowed to request from this client at once.
    pub maximum_allowed_reply_surb_request_size: u32,

    /// Defines the amount of reply surbs that the client is going to request when it runs out while attempting to retransmit packets.
    pub retransmission_reply_surb_request_size: u32,

    /// Defines maximum amount of time the client is going to wait for reply surbs before explicitly asking
    /// for more even though in theory they wouldn't need to.
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_waiting_period: Duration,

    /// Defines maximum amount of time given reply surb is going to be valid for.
    /// This is going to be superseded by key rotation once implemented.
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_age: Duration,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExtendedPacketSize {
    Extended8,
    Extended16,
    Extended32,
}

impl Default for DebugConfig {
    fn default() -> Self {
        DebugConfig {
            average_packet_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            average_ack_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            ack_wait_multiplier: DEFAULT_ACK_WAIT_MULTIPLIER,
            ack_wait_addition: DEFAULT_ACK_WAIT_ADDITION,
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            message_sending_average_delay: DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY,
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
            disable_loop_cover_traffic_stream: false,
            disable_main_poisson_packet_distribution: false,
            use_extended_packet_size: None,
            minimum_reply_surb_storage_threshold: DEFAULT_MINIMUM_REPLY_SURB_STORAGE_THRESHOLD,
            maximum_reply_surb_storage_threshold: DEFAULT_MAXIMUM_REPLY_SURB_STORAGE_THRESHOLD,
            minimum_reply_surb_request_size: DEFAULT_MINIMUM_REPLY_SURB_REQUEST_SIZE,
            maximum_reply_surb_request_size: DEFAULT_MAXIMUM_REPLY_SURB_REQUEST_SIZE,
            maximum_allowed_reply_surb_request_size: DEFAULT_MAXIMUM_ALLOWED_SURB_REQUEST_SIZE,
            retransmission_reply_surb_request_size: DEFAULT_RETRANSMISSION_REPLY_SURB_REQUEST_SIZE,
            maximum_reply_surb_waiting_period: DEFAULT_MAXIMUM_REPLY_SURB_WAITING_PERIOD,
            maximum_reply_surb_age: DEFAULT_MAXIMUM_REPLY_SURB_AGE,
        }
    }
}

impl From<ExtendedPacketSize> for PacketSize {
    fn from(size: ExtendedPacketSize) -> PacketSize {
        match size {
            ExtendedPacketSize::Extended8 => PacketSize::ExtendedPacket8,
            ExtendedPacketSize::Extended16 => PacketSize::ExtendedPacket16,
            ExtendedPacketSize::Extended32 => PacketSize::ExtendedPacket32,
        }
    }
}
