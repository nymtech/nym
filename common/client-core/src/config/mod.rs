// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_config::defaults::NymNetworkDetails;
use nym_config::{NymConfig, OptionalSet, CRED_DB_FILE_NAME};
use nym_sphinx::params::{PacketSize, PacketType};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod old_config_v1_1_13;
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

const DEFAULT_COVER_TRAFFIC_PRIMARY_SIZE_RATIO: f64 = 0.70;

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

const DEFAULT_MAXIMUM_REPLY_SURB_REREQUEST_WAITING_PERIOD: Duration = Duration::from_secs(10);
const DEFAULT_MAXIMUM_REPLY_SURB_DROP_WAITING_PERIOD: Duration = Duration::from_secs(5 * 60);

// 12 hours
const DEFAULT_MAXIMUM_REPLY_SURB_AGE: Duration = Duration::from_secs(12 * 60 * 60);

// 24 hours
const DEFAULT_MAXIMUM_REPLY_KEY_AGE: Duration = Duration::from_secs(24 * 60 * 60);

pub fn missing_string_value() -> String {
    MISSING_VALUE.to_string()
}

pub trait ClientCoreConfigTrait {
    fn get_gateway_endpoint(&self) -> &GatewayEndpointConfig;
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

impl<T> ClientCoreConfigTrait for Config<T> {
    fn get_gateway_endpoint(&self) -> &GatewayEndpointConfig {
        &self.client.gateway_endpoint
    }
}

impl<T> OptionalSet for Config<T> where T: NymConfig {}

impl<T> Config<T> {
    pub fn new<S: Into<String>>(id: S) -> Self
    where
        T: NymConfig,
    {
        Config::default().with_id(id)
    }

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.debug.validate()
    }

    #[must_use]
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self
    where
        T: NymConfig,
    {
        self.client.id = id.into();
        self.set_empty_fields_to_defaults();
        self
    }

    pub fn set_empty_fields_to_defaults(&mut self) -> bool
    where
        T: NymConfig,
    {
        let id = &self.client.id;
        let mut changes_made = false;

        // identity key setting
        if self.client.private_identity_key_file.as_os_str().is_empty() {
            changes_made = true;
            self.client.private_identity_key_file =
                self::Client::<T>::default_private_identity_key_file(id);
        }
        if self.client.public_identity_key_file.as_os_str().is_empty() {
            changes_made = true;
            self.client.public_identity_key_file =
                self::Client::<T>::default_public_identity_key_file(id);
        }

        // encryption key setting
        if self
            .client
            .private_encryption_key_file
            .as_os_str()
            .is_empty()
        {
            changes_made = true;
            self.client.private_encryption_key_file =
                self::Client::<T>::default_private_encryption_key_file(id);
        }
        if self
            .client
            .public_encryption_key_file
            .as_os_str()
            .is_empty()
        {
            changes_made = true;
            self.client.public_encryption_key_file =
                self::Client::<T>::default_public_encryption_key_file(id);
        }

        // shared gateway key setting
        if self.client.gateway_shared_key_file.as_os_str().is_empty() {
            changes_made = true;
            self.client.gateway_shared_key_file =
                self::Client::<T>::default_gateway_shared_key_file(id);
        }

        // ack key setting
        if self.client.ack_key_file.as_os_str().is_empty() {
            changes_made = true;
            self.client.ack_key_file = self::Client::<T>::default_ack_key_file(id);
        }

        if self.client.reply_surb_database_path.as_os_str().is_empty() {
            changes_made = true;
            self.client.reply_surb_database_path =
                self::Client::<T>::default_reply_surb_database_path(id);
        }

        if self.client.database_path.as_os_str().is_empty() {
            changes_made = true;
            self.client.database_path = self::Client::<T>::default_database_path(id);
        }
        changes_made
    }

    pub fn with_disabled_credentials(mut self, disabled_credentials_mode: bool) -> Self {
        self.client.disabled_credentials_mode = disabled_credentials_mode;
        self
    }

    pub fn set_gateway_endpoint(&mut self, gateway_endpoint: GatewayEndpointConfig) {
        self.client.gateway_endpoint = gateway_endpoint;
    }

    pub fn with_gateway_endpoint(mut self, gateway_endpoint: GatewayEndpointConfig) -> Self {
        self.client.gateway_endpoint = gateway_endpoint;
        self
    }

    pub fn with_gateway_id<S: Into<String>>(&mut self, id: S) {
        self.client.gateway_endpoint.gateway_id = id.into();
    }

    pub fn with_custom_nyxd(mut self, urls: Vec<Url>) -> Self {
        self.client.nyxd_urls = urls;
        self
    }

    pub fn set_custom_nyxd(&mut self, nyxd_urls: Vec<Url>) {
        self.client.nyxd_urls = nyxd_urls;
    }

    pub fn with_custom_nym_apis(mut self, nym_api_urls: Vec<Url>) -> Self {
        self.client.nym_api_urls = nym_api_urls;
        self
    }

    pub fn set_custom_nym_apis(&mut self, nym_api_urls: Vec<Url>) {
        self.client.nym_api_urls = nym_api_urls;
    }

    pub fn with_high_default_traffic_volume(mut self, enabled: bool) -> Self {
        if enabled {
            self.set_high_default_traffic_volume();
        }
        self
    }

    pub fn with_packet_type(mut self, packet_type: PacketType) -> Self {
        self.client.packet_type = Some(packet_type);
        self
    }

    pub fn set_high_default_traffic_volume(&mut self) {
        self.debug.traffic.average_packet_delay = Duration::from_millis(10);
        // basically don't really send cover messages
        self.debug.cover_traffic.loop_cover_traffic_average_delay =
            Duration::from_millis(2_000_000);
        // 250 "real" messages / s
        self.debug.traffic.message_sending_average_delay = Duration::from_millis(4);
    }

    pub fn with_disabled_cover_traffic(mut self, disabled: bool) -> Self {
        if disabled {
            self.set_no_cover_traffic()
        }
        self
    }

    pub fn set_no_cover_traffic(&mut self) {
        self.debug.cover_traffic.disable_loop_cover_traffic_stream = true;
        self.debug.traffic.disable_main_poisson_packet_distribution = true;
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
        self.client.nyxd_urls.clone()
    }

    pub fn get_nym_api_endpoints(&self) -> Vec<Url> {
        self.client.nym_api_urls.clone()
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
        self.debug.traffic.average_packet_delay
    }

    pub fn get_average_ack_delay(&self) -> Duration {
        self.debug.acknowledgements.average_ack_delay
    }

    pub fn get_ack_wait_multiplier(&self) -> f64 {
        self.debug.acknowledgements.ack_wait_multiplier
    }

    pub fn get_ack_wait_addition(&self) -> Duration {
        self.debug.acknowledgements.ack_wait_addition
    }

    pub fn get_loop_cover_traffic_average_delay(&self) -> Duration {
        self.debug.cover_traffic.loop_cover_traffic_average_delay
    }

    pub fn get_message_sending_average_delay(&self) -> Duration {
        self.debug.traffic.message_sending_average_delay
    }

    pub fn get_gateway_response_timeout(&self) -> Duration {
        self.debug.gateway_connection.gateway_response_timeout
    }

    pub fn get_topology_refresh_rate(&self) -> Duration {
        self.debug.topology.topology_refresh_rate
    }

    pub fn get_topology_resolution_timeout(&self) -> Duration {
        self.debug.topology.topology_resolution_timeout
    }

    pub fn get_disabled_loop_cover_traffic_stream(&self) -> bool {
        self.debug.cover_traffic.disable_loop_cover_traffic_stream
    }

    pub fn get_disabled_main_poisson_packet_distribution(&self) -> bool {
        self.debug.traffic.disable_main_poisson_packet_distribution
    }

    pub fn get_minimum_reply_surb_storage_threshold(&self) -> usize {
        self.debug.reply_surbs.minimum_reply_surb_storage_threshold
    }

    pub fn get_maximum_reply_surb_storage_threshold(&self) -> usize {
        self.debug.reply_surbs.maximum_reply_surb_storage_threshold
    }

    pub fn get_minimum_reply_surb_request_size(&self) -> u32 {
        self.debug.reply_surbs.minimum_reply_surb_request_size
    }

    pub fn get_maximum_reply_surb_request_size(&self) -> u32 {
        self.debug.reply_surbs.maximum_reply_surb_request_size
    }

    pub fn get_maximum_allowed_reply_surb_request_size(&self) -> u32 {
        self.debug
            .reply_surbs
            .maximum_allowed_reply_surb_request_size
    }

    pub fn get_maximum_reply_surb_rerequest_waiting_period(&self) -> Duration {
        self.debug
            .reply_surbs
            .maximum_reply_surb_rerequest_waiting_period
    }

    pub fn get_maximum_reply_surb_drop_waiting_period(&self) -> Duration {
        self.debug
            .reply_surbs
            .maximum_reply_surb_drop_waiting_period
    }

    pub fn get_maximum_reply_surb_age(&self) -> Duration {
        self.debug.reply_surbs.maximum_reply_surb_age
    }

    pub fn get_maximum_reply_key_age(&self) -> Duration {
        self.debug.reply_surbs.maximum_reply_key_age
    }

    pub fn get_packet_type(&self) -> PacketType {
        self.client.packet_type.unwrap_or(PacketType::Mix)
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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GatewayEndpointConfig {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(
        gateway_id: String,
        gateway_owner: String,
        gateway_listener: String,
    ) -> GatewayEndpointConfig {
        GatewayEndpointConfig {
            gateway_id,
            gateway_owner,
            gateway_listener,
        }
    }
}

impl From<nym_topology::gateway::Node> for GatewayEndpointConfig {
    fn from(node: nym_topology::gateway::Node) -> GatewayEndpointConfig {
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
    pub version: String,

    /// ID specifies the human readable ID of this particular client.
    pub id: String,

    /// Indicates whether this client is running in a disabled credentials mode, thus attempting
    /// to claim bandwidth without presenting bandwidth credentials.
    #[serde(default)]
    pub disabled_credentials_mode: bool,

    /// Addresses to nyxd validators via which the client can communicate with the chain.
    #[serde(alias = "validator_urls")]
    pub nyxd_urls: Vec<Url>,

    /// Addresses to APIs running on validator from which the client gets the view of the network.
    #[serde(alias = "validator_api_urls")]
    pub nym_api_urls: Vec<Url>,

    /// Path to file containing private identity key.
    pub private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    pub public_identity_key_file: PathBuf,

    /// Path to file containing private encryption key.
    pub private_encryption_key_file: PathBuf,

    /// Path to file containing public encryption key.
    pub public_encryption_key_file: PathBuf,

    /// Path to file containing shared key derived with the specified gateway that is used
    /// for all communication with it.
    pub gateway_shared_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    pub ack_key_file: PathBuf,

    /// Information regarding how the client should send data to gateway.
    pub gateway_endpoint: GatewayEndpointConfig,

    /// Path to the database containing bandwidth credentials of this client.
    pub database_path: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    // this was set to use #[serde(default)] for the purposes of compatibility for multi-surbs introduced in 1.1.4.
    // if you're reading this message and we have already introduced some breaking changes, feel free
    // to remove that attribute since at this point the client configs should have gotten regenerated
    #[serde(default)]
    pub reply_surb_database_path: PathBuf,

    /// nym_home_directory specifies absolute path to the home nym Clients directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    pub nym_root_directory: PathBuf,

    #[serde(skip)]
    pub super_struct: PhantomData<T>,

    pub packet_type: Option<PacketType>,
}

impl<T: NymConfig> Default for Client<T> {
    fn default() -> Self {
        let network = NymNetworkDetails::new_mainnet();
        let nyxd_urls = network
            .endpoints
            .iter()
            .map(|validator| validator.nyxd_url())
            .collect();
        let nym_api_urls = network
            .endpoints
            .iter()
            .filter_map(|validator| validator.api_url())
            .collect::<Vec<_>>();

        if nym_api_urls.is_empty() {
            panic!("we do not have any default nym-api urls available!")
        }

        // there must be explicit checks for whether id is not empty later
        Client {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: "".to_string(),
            disabled_credentials_mode: true,
            nyxd_urls,
            nym_api_urls,
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
            packet_type: Default::default(),
        }
    }
}

impl<T: NymConfig> Client<T> {
    fn default_private_identity_key_file(id: &str) -> PathBuf {
        T::default_data_directory(id).join("private_identity.pem")
    }

    fn default_public_identity_key_file(id: &str) -> PathBuf {
        T::default_data_directory(id).join("public_identity.pem")
    }

    fn default_private_encryption_key_file(id: &str) -> PathBuf {
        T::default_data_directory(id).join("private_encryption.pem")
    }

    fn default_public_encryption_key_file(id: &str) -> PathBuf {
        T::default_data_directory(id).join("public_encryption.pem")
    }

    fn default_gateway_shared_key_file(id: &str) -> PathBuf {
        T::default_data_directory(id).join("gateway_shared.pem")
    }

    fn default_ack_key_file(id: &str) -> PathBuf {
        T::default_data_directory(id).join("ack_key.pem")
    }

    fn default_reply_surb_database_path(id: &str) -> PathBuf {
        T::default_data_directory(id).join("persistent_reply_store.sqlite")
    }

    fn default_database_path(id: &str) -> PathBuf {
        T::default_data_directory(id).join(CRED_DB_FILE_NAME)
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Logging {}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Traffic {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent packet is going to be delayed at any given mix node.
    /// So for a packet going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[serde(with = "humantime_serde")]
    pub average_packet_delay: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    #[serde(with = "humantime_serde")]
    pub message_sending_average_delay: Duration,

    /// Controls whether the main packet stream constantly produces packets according to the predefined
    /// poisson distribution.
    pub disable_main_poisson_packet_distribution: bool,

    /// Specifies the packet size used for sent messages.
    /// Do not override it unless you understand the consequences of that change.
    pub primary_packet_size: PacketSize,

    /// Specifies the optional auxiliary packet size for optimizing message streams.
    /// Note that its use decreases overall anonymity.
    /// Do not set it it unless you understand the consequences of that change.
    pub secondary_packet_size: Option<PacketSize>,
}

impl Traffic {
    pub fn validate(&self) -> bool {
        if let Some(secondary_packet_size) = self.secondary_packet_size {
            if secondary_packet_size == PacketSize::AckPacket
                || secondary_packet_size == self.primary_packet_size
            {
                return false;
            }
        }
        true
    }
}

impl Default for Traffic {
    fn default() -> Self {
        Traffic {
            average_packet_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            message_sending_average_delay: DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY,
            disable_main_poisson_packet_distribution: false,
            primary_packet_size: PacketSize::RegularPacket,
            secondary_packet_size: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CoverTraffic {
    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    #[serde(with = "humantime_serde")]
    pub loop_cover_traffic_average_delay: Duration,

    /// Specifies the ratio of `primary_packet_size` to `secondary_packet_size` used in cover traffic.
    /// Only applicable if `secondary_packet_size` is enabled.
    pub cover_traffic_primary_size_ratio: f64,

    /// Controls whether the dedicated loop cover traffic stream should be enabled.
    /// (and sending packets, on average, every [Self::loop_cover_traffic_average_delay])
    pub disable_loop_cover_traffic_stream: bool,
}

impl Default for CoverTraffic {
    fn default() -> Self {
        CoverTraffic {
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            cover_traffic_primary_size_ratio: DEFAULT_COVER_TRAFFIC_PRIMARY_SIZE_RATIO,
            disable_loop_cover_traffic_stream: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GatewayConnection {
    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    #[serde(with = "humantime_serde")]
    pub gateway_response_timeout: Duration,
}

impl Default for GatewayConnection {
    fn default() -> Self {
        GatewayConnection {
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Acknowledgements {
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
}

impl Default for Acknowledgements {
    fn default() -> Self {
        Acknowledgements {
            average_ack_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            ack_wait_multiplier: DEFAULT_ACK_WAIT_MULTIPLIER,
            ack_wait_addition: DEFAULT_ACK_WAIT_ADDITION,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Topology {
    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    #[serde(with = "humantime_serde")]
    pub topology_refresh_rate: Duration,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    #[serde(with = "humantime_serde")]
    pub topology_resolution_timeout: Duration,

    /// Specifies whether the client should not refresh the network topology after obtaining
    /// the first valid instance.
    /// Supersedes `topology_refresh_rate_ms`.
    pub disable_refreshing: bool,
}

impl Default for Topology {
    fn default() -> Self {
        Topology {
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
            disable_refreshing: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReplySurbs {
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

    /// Defines maximum amount of time the client is going to wait for reply surbs before explicitly asking
    /// for more even though in theory they wouldn't need to.
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_rerequest_waiting_period: Duration,

    /// Defines maximum amount of time the client is going to wait for reply surbs before
    /// deciding it's never going to get them and would drop all pending messages
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_drop_waiting_period: Duration,

    /// Defines maximum amount of time given reply surb is going to be valid for.
    /// This is going to be superseded by key rotation once implemented.
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_age: Duration,

    /// Defines maximum amount of time given reply key is going to be valid for.
    /// This is going to be superseded by key rotation once implemented.
    #[serde(with = "humantime_serde")]
    pub maximum_reply_key_age: Duration,
}

impl Default for ReplySurbs {
    fn default() -> Self {
        ReplySurbs {
            minimum_reply_surb_storage_threshold: DEFAULT_MINIMUM_REPLY_SURB_STORAGE_THRESHOLD,
            maximum_reply_surb_storage_threshold: DEFAULT_MAXIMUM_REPLY_SURB_STORAGE_THRESHOLD,
            minimum_reply_surb_request_size: DEFAULT_MINIMUM_REPLY_SURB_REQUEST_SIZE,
            maximum_reply_surb_request_size: DEFAULT_MAXIMUM_REPLY_SURB_REQUEST_SIZE,
            maximum_allowed_reply_surb_request_size: DEFAULT_MAXIMUM_ALLOWED_SURB_REQUEST_SIZE,
            maximum_reply_surb_rerequest_waiting_period:
                DEFAULT_MAXIMUM_REPLY_SURB_REREQUEST_WAITING_PERIOD,
            maximum_reply_surb_drop_waiting_period: DEFAULT_MAXIMUM_REPLY_SURB_DROP_WAITING_PERIOD,
            maximum_reply_surb_age: DEFAULT_MAXIMUM_REPLY_SURB_AGE,
            maximum_reply_key_age: DEFAULT_MAXIMUM_REPLY_KEY_AGE,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugConfig {
    /// Defines all configuration options related to traffic streams.
    pub traffic: Traffic,

    /// Defines all configuration options related to cover traffic stream(s).
    pub cover_traffic: CoverTraffic,

    /// Defines all configuration options related to the gateway connection.
    pub gateway_connection: GatewayConnection,

    /// Defines all configuration options related to acknowledgements, such as delays or wait timeouts.
    pub acknowledgements: Acknowledgements,

    /// Defines all configuration options related topology, such as refresh rates or timeouts.
    pub topology: Topology,

    /// Defines all configuration options related to reply SURBs.
    pub reply_surbs: ReplySurbs,
}

impl DebugConfig {
    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.traffic.validate()
    }
}

// it could be derived, sure, but I'd rather have an explicit implementation in case we had to change
// something manually at some point
#[allow(clippy::derivable_impls)]
impl Default for DebugConfig {
    fn default() -> Self {
        DebugConfig {
            traffic: Default::default(),
            cover_traffic: Default::default(),
            gateway_connection: Default::default(),
            acknowledgements: Default::default(),
            topology: Default::default(),
            reply_surbs: Default::default(),
        }
    }
}
