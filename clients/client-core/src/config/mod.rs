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

use config::NymConfig;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time;
use std::time::Duration;

pub mod persistence;

pub const MISSING_VALUE: &str = "MISSING VALUE";

// 'CLIENT'
const DEFAULT_DIRECTORY_SERVER: &str = "https://directory.nymtech.net";
// 'DEBUG'
// where applicable, the below are defined in milliseconds
const DEFAULT_ACK_WAIT_MULTIPLIER: f64 = 1.5;

// all delays are in milliseconds
const DEFAULT_ACK_WAIT_ADDITION: u64 = 1_500;
const DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY: u64 = 1000;
const DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY: u64 = 100;
const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(100);
const DEFAULT_TOPOLOGY_REFRESH_RATE: u64 = 30_000;
const DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT: u64 = 5_000;
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: u64 = 1_500;
const DEFAULT_VPN_KEY_REUSE_LIMIT: usize = 1000;

const ZERO_DELAY: Duration = Duration::from_nanos(0);

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

        self.client.id = id;
    }

    pub fn with_gateway_id<S: Into<String>>(&mut self, id: S) {
        self.client.gateway_id = id.into();
    }

    pub fn with_gateway_listener<S: Into<String>>(&mut self, gateway_listener: S) {
        self.client.gateway_listener = gateway_listener.into();
    }

    pub fn with_custom_directory<S: Into<String>>(&mut self, directory_server: S) {
        self.client.directory_server = directory_server.into();
    }

    pub fn set_high_default_traffic_volume(&mut self) {
        self.debug.average_packet_delay = Duration::from_millis(10);
        self.debug.loop_cover_traffic_average_delay = 20; // 50 cover messages / s
        self.debug.message_sending_average_delay = 5; // 200 "real" messages / s
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

    pub fn get_directory_server(&self) -> String {
        self.client.directory_server.clone()
    }

    pub fn get_gateway_id(&self) -> String {
        self.client.gateway_id.clone()
    }

    pub fn get_gateway_listener(&self) -> String {
        self.client.gateway_listener.clone()
    }

    // Debug getters
    pub fn get_average_packet_delay(&self) -> time::Duration {
        if self.client.vpn_mode {
            ZERO_DELAY
        } else {
            self.debug.average_packet_delay
        }
    }

    pub fn get_average_ack_delay(&self) -> time::Duration {
        if self.client.vpn_mode {
            ZERO_DELAY
        } else {
            self.debug.average_ack_delay
        }
    }

    pub fn get_ack_wait_multiplier(&self) -> f64 {
        self.debug.ack_wait_multiplier
    }

    pub fn get_ack_wait_addition(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.ack_wait_addition)
    }

    pub fn get_loop_cover_traffic_average_delay(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.loop_cover_traffic_average_delay)
    }

    pub fn get_message_sending_average_delay(&self) -> time::Duration {
        if self.client.vpn_mode {
            ZERO_DELAY
        } else {
            time::Duration::from_millis(self.debug.message_sending_average_delay)
        }
    }

    pub fn get_gateway_response_timeout(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.gateway_response_timeout)
    }

    pub fn get_topology_refresh_rate(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.topology_refresh_rate)
    }

    pub fn get_topology_resolution_timeout(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.topology_resolution_timeout)
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
                    .unwrap_or_else(|| DEFAULT_VPN_KEY_REUSE_LIMIT),
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
#[serde(deny_unknown_fields)]
pub struct Client<T> {
    /// Version of the client for which this configuration was created.
    #[serde(default = "missing_string_value")]
    version: String,

    /// ID specifies the human readable ID of this particular client.
    id: String,

    /// URL to the directory server.
    directory_server: String,

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
            directory_server: DEFAULT_DIRECTORY_SERVER.to_string(),
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
    /// The provided value is interpreted as milliseconds.
    average_packet_delay: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// sent acknowledgement is going to be delayed at any given mix node.
    /// So for an ack going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    /// The provided value is interpreted as milliseconds.
    average_ack_delay: Duration,

    /// Value multiplied with the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 1.
    ack_wait_multiplier: f64,

    /// Value added to the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 0.
    /// The provided value is interpreted as milliseconds.
    ack_wait_addition: u64,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    /// The provided value is interpreted as milliseconds.
    loop_cover_traffic_average_delay: u64,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    /// The provided value is interpreted as milliseconds.
    message_sending_average_delay: u64,

    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    /// The provided value is interpreted as milliseconds.
    gateway_response_timeout: u64,

    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    /// The provided value is interpreted as milliseconds.
    topology_refresh_rate: u64,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    /// The provided value is interpreted as milliseconds.
    topology_resolution_timeout: u64,

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
