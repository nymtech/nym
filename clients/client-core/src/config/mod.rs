// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use config::{deserialize_duration, deserialize_validators, NymConfig};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::Duration;

pub mod persistence;

pub const MISSING_VALUE: &str = "MISSING VALUE";

// 'CLIENT'
pub const DEFAULT_VALIDATOR_REST_ENDPOINTS: &[&str] = &[
    "http://testnet-finney-validator.nymtech.net:1317",
    "http://testnet-finney-validator2.nymtech.net:1317",
    "http://mixnet.club:1317",
];
pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = "hal1k0jntykt7e4g3y88ltc60czgjuqdy4c9c6gv94";

// 'DEBUG'
const DEFAULT_ACK_WAIT_MULTIPLIER: f64 = 1.5;

const DEFAULT_ACK_WAIT_ADDITION: Duration = Duration::from_millis(1_500);
const DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(20);
const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(50);
const DEFAULT_TOPOLOGY_REFRESH_RATE: Duration = Duration::from_secs(5 * 60); // every 5min
const DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT: Duration = Duration::from_millis(5_000);
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_millis(1_500);
const DEFAULT_VPN_KEY_REUSE_LIMIT: usize = 1000;

const ZERO_DELAY: Duration = Duration::from_nanos(0);

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

        self.client.id = id;
    }

    pub fn with_gateway_id<S: Into<String>>(&mut self, id: S) {
        self.client.gateway_id = id.into();
    }

    pub fn with_gateway_listener<S: Into<String>>(&mut self, gateway_listener: S) {
        self.client.gateway_listener = gateway_listener.into();
    }

    pub fn set_custom_validators(&mut self, validators: Vec<String>) {
        self.client.validator_rest_urls = validators;
    }

    pub fn set_mixnet_contract<S: Into<String>>(&mut self, contract_address: S) {
        self.client.mixnet_contract_address = contract_address.into();
    }

    pub fn set_high_default_traffic_volume(&mut self) {
        self.debug.average_packet_delay = Duration::from_millis(10);
        self.debug.loop_cover_traffic_average_delay = Duration::from_millis(2000000); // basically don't really send cover messages
        self.debug.message_sending_average_delay = Duration::from_millis(4); // 250 "real" messages / s
    }

    pub fn set_vpn_mode(&mut self, vpn_mode: bool) {
        self.client.vpn_mode = vpn_mode;
    }

    pub fn set_vpn_key_reuse_limit(&mut self, reuse_limit: usize) {
        self.debug.vpn_key_reuse_limit = Some(reuse_limit)
    }

    pub fn set_custom_version(&mut self, version: &str) {
        self.client.version = version.to_string();
    }

    pub fn get_id(&self) -> String {
        self.client.id.clone()
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

    pub fn get_validator_rest_endpoints(&self) -> Vec<String> {
        self.client.validator_rest_urls.clone()
    }

    pub fn get_validator_mixnet_contract_address(&self) -> String {
        self.client.mixnet_contract_address.clone()
    }

    pub fn get_gateway_id(&self) -> String {
        self.client.gateway_id.clone()
    }

    pub fn get_gateway_listener(&self) -> String {
        self.client.gateway_listener.clone()
    }

    // Debug getters
    pub fn get_average_packet_delay(&self) -> Duration {
        if self.client.vpn_mode {
            ZERO_DELAY
        } else {
            self.debug.average_packet_delay
        }
    }

    pub fn get_average_ack_delay(&self) -> Duration {
        if self.client.vpn_mode {
            ZERO_DELAY
        } else {
            self.debug.average_ack_delay
        }
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
        if self.client.vpn_mode {
            ZERO_DELAY
        } else {
            self.debug.message_sending_average_delay
        }
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

    pub fn get_vpn_mode(&self) -> bool {
        self.client.vpn_mode
    }

    pub fn get_vpn_key_reuse_limit(&self) -> Option<usize> {
        match self.get_vpn_mode() {
            false => None,
            true => Some(
                self.debug
                    .vpn_key_reuse_limit
                    .unwrap_or(DEFAULT_VPN_KEY_REUSE_LIMIT),
            ),
        }
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

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Client<T> {
    /// Version of the client for which this configuration was created.
    #[serde(default = "missing_string_value")]
    version: String,

    /// ID specifies the human readable ID of this particular client.
    id: String,

    /// URL to the validator server for obtaining network topology.
    #[serde(
        deserialize_with = "deserialize_validators",
        default = "missing_vec_string_value",
        alias = "validator_rest_url"
    )]
    validator_rest_urls: Vec<String>,

    /// Address of the validator contract managing the network.
    #[serde(default = "missing_string_value")]
    mixnet_contract_address: String,

    /// Special mode of the system such that all messages are sent as soon as they are received
    /// and no cover traffic is generated. If set all message delays are set to 0 and overwriting
    /// 'Debug' values will have no effect.
    #[serde(default)]
    vpn_mode: bool,

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

    /// gateway_id specifies ID of the gateway to which the client should send messages.
    /// If initially omitted, a random gateway will be chosen from the available topology.
    gateway_id: String,

    /// Address of the gateway listener to which all client requests should be sent.
    gateway_listener: String,

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
            validator_rest_urls: default_validator_rest_endpoints(),
            mixnet_contract_address: DEFAULT_MIXNET_CONTRACT_ADDRESS.to_string(),
            vpn_mode: false,
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
            private_encryption_key_file: Default::default(),
            public_encryption_key_file: Default::default(),
            gateway_shared_key_file: Default::default(),
            ack_key_file: Default::default(),
            reply_encryption_key_store_path: Default::default(),
            gateway_id: "".to_string(),
            gateway_listener: "".to_string(),
            nym_root_directory: T::default_root_directory(),
            super_struct: Default::default(),
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

    fn default_reply_encryption_key_store_path(id: &str) -> PathBuf {
        T::default_data_directory(id).join("reply_key_store")
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
#[serde(default, deny_unknown_fields)]
pub struct Debug {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent packet is going to be delayed at any given mix node.
    /// So for a packet going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    average_packet_delay: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// sent acknowledgement is going to be delayed at any given mix node.
    /// So for an ack going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    average_ack_delay: Duration,

    /// Value multiplied with the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 1.
    ack_wait_multiplier: f64,

    /// Value added to the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 0.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    ack_wait_addition: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    loop_cover_traffic_average_delay: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    message_sending_average_delay: Duration,

    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    gateway_response_timeout: Duration,

    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    topology_refresh_rate: Duration,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    topology_resolution_timeout: Duration,

    /// If the mode of the client is set to VPN it specifies number of packets created with the
    /// same initial secret until it gets rotated.
    vpn_key_reuse_limit: Option<usize>,
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
            vpn_key_reuse_limit: None,
        }
    }
}
