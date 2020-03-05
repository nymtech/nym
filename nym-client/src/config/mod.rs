use crate::config::template::config_template;
use config::NymConfig;
use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;
use std::time;

pub mod persistence;
mod template;

// 'CLIENT'
const DEFAULT_LISTENING_PORT: u16 = 9001;

// 'DEBUG'
// where applicable, the below are defined in milliseconds
const DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY: u64 = 1000;
const DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY: u64 = 500;
const DEFAULT_AVERAGE_PACKET_DELAY: u64 = 200;
const DEFAULT_FETCH_MESSAGES_DELAY: u64 = 1000;
const DEFAULT_TOPOLOGY_REFRESH_RATE: u64 = 10_000;
const DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT: u64 = 5_000;
const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: u64 = 10_000; // 10s
const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: u64 = 300_000; // 5min

const DEFAULT_NUMBER_OF_HEALTHCHECK_TEST_PACKETS: u64 = 2;
const DEFAULT_NODE_SCORE_THRESHOLD: f64 = 0.0;

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub enum SocketType {
    TCP,
    WebSocket,
    None,
}

impl SocketType {
    pub fn from_string<S: Into<String>>(val: S) -> Self {
        let mut upper = val.into();
        upper.make_ascii_uppercase();
        match upper.as_ref() {
            "TCP" => SocketType::TCP,
            "WEBSOCKET" | "WS" => SocketType::WebSocket,
            _ => SocketType::None,
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    client: Client,
    socket: Socket,

    #[serde(default)]
    logging: Logging,
    #[serde(default)]
    debug: Debug,
}

impl NymConfig for Config {
    fn template() -> &'static str {
        config_template()
    }

    fn config_file_name() -> String {
        "config.toml".to_string()
    }

    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("clients")
    }

    fn root_directory(&self) -> PathBuf {
        self.client.nym_root_directory.clone()
    }

    fn config_directory(&self) -> PathBuf {
        self.client
            .nym_root_directory
            .join(&self.client.id)
            .join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.client
            .nym_root_directory
            .join(&self.client.id)
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
        if self.client.private_identity_key_file.as_os_str().is_empty() {
            self.client.private_identity_key_file =
                self::Client::default_private_identity_key_file(&id);
        }
        if self.client.public_identity_key_file.as_os_str().is_empty() {
            self.client.public_identity_key_file =
                self::Client::default_public_identity_key_file(&id);
        }
        self.client.id = id;
        self
    }

    pub fn with_provider_id<S: Into<String>>(mut self, id: S) -> Self {
        self.client.provider_id = id.into();
        self
    }

    pub fn with_provider_auth_token<S: Into<String>>(mut self, token: S) -> Self {
        self.client.provider_authtoken = Some(token.into());
        self
    }

    pub fn with_custom_directory<S: Into<String>>(mut self, directory_server: S) -> Self {
        self.client.directory_server = directory_server.into();
        self
    }

    pub fn with_socket(mut self, socket_type: SocketType) -> Self {
        self.socket.socket_type = socket_type;
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.socket.listening_port = port;
        self
    }

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }

    pub fn get_private_identity_key_file(&self) -> PathBuf {
        self.client.private_identity_key_file.clone()
    }

    pub fn get_public_identity_key_file(&self) -> PathBuf {
        self.client.public_identity_key_file.clone()
    }

    pub fn get_directory_server(&self) -> String {
        self.client.directory_server.clone()
    }

    pub fn get_provider_id(&self) -> String {
        self.client.provider_id.clone()
    }

    pub fn get_provider_auth_token(&self) -> Option<String> {
        self.client.provider_authtoken.clone()
    }

    pub fn get_socket_type(&self) -> SocketType {
        self.socket.socket_type
    }

    pub fn get_listening_port(&self) -> u16 {
        self.socket.listening_port
    }

    // Debug getters
    pub fn get_average_packet_delay(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.average_packet_delay)
    }

    pub fn get_loop_cover_traffic_average_delay(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.loop_cover_traffic_average_delay)
    }

    pub fn get_fetch_message_delay(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.fetch_message_delay)
    }

    pub fn get_message_sending_average_delay(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.message_sending_average_delay)
    }

    pub fn get_rate_compliant_cover_messages_disabled(&self) -> bool {
        self.debug.rate_compliant_cover_messages_disabled
    }

    pub fn get_topology_refresh_rate(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.topology_refresh_rate)
    }

    pub fn get_topology_resolution_timeout(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.topology_resolution_timeout)
    }

    pub fn get_number_of_healthcheck_test_packets(&self) -> u64 {
        self.debug.number_of_healthcheck_test_packets
    }

    pub fn get_node_score_threshold(&self) -> f64 {
        self.debug.node_score_threshold
    }

    pub fn get_packet_forwarding_initial_backoff(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.packet_forwarding_initial_backoff)
    }

    pub fn get_packet_forwarding_maximum_backoff(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.packet_forwarding_maximum_backoff)
    }
}

fn de_option_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    /// ID specifies the human readable ID of this particular client.
    id: String,

    /// URL to the directory server.
    directory_server: String,

    /// Path to file containing private identity key.
    private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    public_identity_key_file: PathBuf,

    /// provider_id specifies ID of the provider to which the client should send messages.
    /// If initially omitted, a random provider will be chosen from the available topology.
    provider_id: String,

    /// A provider specific, optional, base58 stringified authentication token used for
    /// communication with particular provider.
    #[serde(deserialize_with = "de_option_string")]
    provider_authtoken: Option<String>,

    /// nym_home_directory specifies absolute path to the home nym Clients directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,
}

impl Default for Client {
    fn default() -> Self {
        // there must be explicit checks for whether id is not empty later
        Client {
            id: "".to_string(),
            directory_server: Self::default_directory_server(),
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
            provider_id: "".to_string(),
            provider_authtoken: None,
            nym_root_directory: Config::default_root_directory(),
        }
    }
}

impl Client {
    fn default_directory_server() -> String {
        #[cfg(feature = "qa")]
        return "https://qa-directory.nymtech.net".to_string();
        #[cfg(feature = "local")]
        return "http://localhost:8080".to_string();

        "https://directory.nymtech.net".to_string()
    }

    fn default_private_identity_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(Some(id)).join("private_identity.pem")
    }

    fn default_public_identity_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(Some(id)).join("public_identity.pem")
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Socket {
    socket_type: SocketType,
    listening_port: u16,
}

impl Default for Socket {
    fn default() -> Self {
        Socket {
            socket_type: SocketType::None,
            listening_port: DEFAULT_LISTENING_PORT,
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
#[serde(deny_unknown_fields)]
pub struct Debug {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent packet is going to be delayed at any given mix node.
    /// So for a packet going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    /// The provided value is interpreted as milliseconds.
    average_packet_delay: u64,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    /// The provided value is interpreted as milliseconds.
    loop_cover_traffic_average_delay: u64,

    /// The uniform delay every which clients are querying the providers for received packets.
    /// The provided value is interpreted as milliseconds.
    fetch_message_delay: u64,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    /// The provided value is interpreted as milliseconds.
    message_sending_average_delay: u64,

    /// Whether loop cover messages should be sent to respect message_sending_rate.
    /// In the case of it being disabled and not having enough real traffic
    /// waiting to be sent the actual sending rate is going be lower than the desired value
    /// thus decreasing the anonymity.
    rate_compliant_cover_messages_disabled: bool,

    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    /// The provided value is interpreted as milliseconds.
    topology_refresh_rate: u64,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    /// The provided value is interpreted as milliseconds.
    topology_resolution_timeout: u64,

    /// How many packets should be sent through each path during the healthcheck
    number_of_healthcheck_test_packets: u64,

    /// In the current healthcheck implementation, threshold indicating percentage of packets
    /// node received during healthcheck. Node's score must be above that value to be
    /// considered healthy.
    node_score_threshold: f64,

    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    /// The provided value is interpreted as milliseconds.
    packet_forwarding_initial_backoff: u64,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    /// The provided value is interpreted as milliseconds.
    packet_forwarding_maximum_backoff: u64,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            average_packet_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            fetch_message_delay: DEFAULT_FETCH_MESSAGES_DELAY,
            message_sending_average_delay: DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY,
            rate_compliant_cover_messages_disabled: false,
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
            number_of_healthcheck_test_packets: DEFAULT_NUMBER_OF_HEALTHCHECK_TEST_PACKETS,
            node_score_threshold: DEFAULT_NODE_SCORE_THRESHOLD,
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
        }
    }
}

#[cfg(test)]
mod client_config {
    use super::*;

    #[test]
    fn after_saving_default_config_the_loaded_one_is_identical() {
        // need to figure out how to do something similar but without touching the disk
        // or the file system at all...
        let temp_location = tempfile::tempdir().unwrap().path().join("config.toml");
        let default_config = Config::default().with_id("foomp".to_string());
        default_config
            .save_to_file(Some(temp_location.clone()))
            .unwrap();

        let loaded_config = Config::load_from_file(Some(temp_location), None).unwrap();

        assert_eq!(default_config, loaded_config);
    }
}
