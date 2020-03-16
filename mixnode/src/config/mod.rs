use crate::config::template::config_template;
use config::NymConfig;
use log::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time;

pub mod persistence;
mod template;

// 'MIXNODE'
const DEFAULT_LISTENING_PORT: u16 = 1789;

// 'DEBUG'
// where applicable, the below are defined in milliseconds
const DEFAULT_PRESENCE_SENDING_DELAY: u64 = 1500;
const DEFAULT_METRICS_SENDING_DELAY: u64 = 1000;
const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: u64 = 10_000; // 10s
const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: u64 = 300_000; // 5min

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    mixnode: MixNode,

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
            .join("mixnodes")
    }

    fn root_directory(&self) -> PathBuf {
        self.mixnode.nym_root_directory.clone()
    }

    fn config_directory(&self) -> PathBuf {
        self.mixnode
            .nym_root_directory
            .join(&self.mixnode.id)
            .join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.mixnode
            .nym_root_directory
            .join(&self.mixnode.id)
            .join("data")
    }
}

impl Config {
    pub fn new<S: Into<String>>(id: S, layer: u64) -> Self {
        Config::default().with_id(id).with_layer(layer)
    }

    // builder methods
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        let id = id.into();
        if self.mixnode.private_sphinx_key_file.as_os_str().is_empty() {
            self.mixnode.private_sphinx_key_file =
                self::MixNode::default_private_sphinx_key_file(&id);
        }
        if self.mixnode.public_sphinx_key_file.as_os_str().is_empty() {
            self.mixnode.public_sphinx_key_file =
                self::MixNode::default_public_sphinx_key_file(&id);
        }
        self.mixnode.id = id;
        self
    }

    pub fn with_layer(mut self, layer: u64) -> Self {
        self.mixnode.layer = layer;
        self
    }

    pub fn with_location<S: Into<String>>(mut self, location: S) -> Self {
        self.mixnode.location = location.into();
        self
    }

    // if you want to use distinct servers for metrics and presence
    // you need to do so in the config.toml file.
    pub fn with_custom_directory<S: Into<String>>(mut self, directory_server: S) -> Self {
        let directory_server_string = directory_server.into();
        self.debug.presence_directory_server = directory_server_string.clone();
        self.debug.metrics_directory_server = directory_server_string;
        self
    }

    pub fn with_listening_host<S: Into<String>>(mut self, host: S) -> Self {
        // see if the provided `host` is just an ip address or ip:port
        let host = host.into();

        // is it ip:port?
        match SocketAddr::from_str(host.as_ref()) {
            Ok(socket_addr) => {
                self.mixnode.listening_address = socket_addr;
                self
            }
            // try just for ip
            Err(_) => match IpAddr::from_str(host.as_ref()) {
                Ok(ip_addr) => {
                    self.mixnode.listening_address.set_ip(ip_addr);
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

    pub fn with_listening_port(mut self, port: u16) -> Self {
        self.mixnode.listening_address.set_port(port);
        self
    }

    pub fn with_announce_host<S: Into<String>>(mut self, host: S) -> Self {
        // this is slightly more complicated as we store announce information as String,
        // since it might not necessarily be a valid SocketAddr (say `nymtech.net:8080` is a valid
        // announce address, yet invalid SocketAddr`

        // first lets see if we received host:port or just host part of an address
        let host = host.into();
        let split_host: Vec<_> = host.split(':').collect();
        match split_host.len() {
            1 => {
                // we provided only 'host' part so we are going to reuse existing port
                self.mixnode.announce_address =
                    format!("{}:{}", host, self.mixnode.listening_address.port());
                self
            }
            2 => {
                // we provided 'host:port' so just put the whole thing there
                self.mixnode.announce_address = host;
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

    pub fn announce_host_from_listening_host(mut self) -> Self {
        self.mixnode.announce_address = self.mixnode.listening_address.to_string();
        self
    }

    pub fn with_announce_port(mut self, port: u16) -> Self {
        let current_host: Vec<_> = self.mixnode.announce_address.split(':').collect();
        debug_assert_eq!(current_host.len(), 2);
        self.mixnode.announce_address = format!("{}:{}", current_host[0], port);
        self
    }

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }

    pub fn get_location(&self) -> String {
        self.mixnode.location.clone()
    }

    pub fn get_private_sphinx_key_file(&self) -> PathBuf {
        self.mixnode.private_sphinx_key_file.clone()
    }

    pub fn get_public_sphinx_key_file(&self) -> PathBuf {
        self.mixnode.public_sphinx_key_file.clone()
    }

    pub fn get_presence_directory_server(&self) -> String {
        self.debug.presence_directory_server.clone()
    }

    pub fn get_presence_sending_delay(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.presence_sending_delay)
    }

    pub fn get_metrics_directory_server(&self) -> String {
        self.debug.metrics_directory_server.clone()
    }

    pub fn get_metrics_sending_delay(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.metrics_sending_delay)
    }

    pub fn get_layer(&self) -> u64 {
        self.mixnode.layer
    }

    pub fn get_listening_address(&self) -> SocketAddr {
        self.mixnode.listening_address
    }

    pub fn get_announce_address(&self) -> String {
        self.mixnode.announce_address.clone()
    }

    pub fn get_packet_forwarding_initial_backoff(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.packet_forwarding_initial_backoff)
    }

    pub fn get_packet_forwarding_maximum_backoff(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.packet_forwarding_maximum_backoff)
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixNode {
    /// ID specifies the human readable ID of this particular mixnode.
    id: String,

    /// Completely optional value specifying geographical location of this particular node.
    /// Currently it's used entirely for debug purposes, as there are no mechanisms implemented
    /// to verify correctness of the information provided. However, feel free to fill in
    /// this field with as much accuracy as you wish to share.
    location: String,

    /// Layer of this particular mixnode determining its position in the network.
    layer: u64,

    /// Socket address to which this mixnode will bind to and will be listening for packets.
    listening_address: SocketAddr,

    /// Optional address announced to the directory server for the clients to connect to.
    /// It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
    /// later on by using name resolvable with a DNS query, such as `nymtech.net:8080`.
    /// Additionally a custom port can be provided, so both `nymtech.net:8080` and `nymtech.net`
    /// are valid announce addresses, while the later will default to whatever port is used for
    /// `listening_address`.
    announce_address: String,

    /// Path to file containing private sphinx key.
    private_sphinx_key_file: PathBuf,

    /// Path to file containing public sphinx key.
    public_sphinx_key_file: PathBuf,

    /// nym_home_directory specifies absolute path to the home nym MixNodes directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,
}

impl MixNode {
    fn default_private_sphinx_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(Some(id)).join("private_sphinx.pem")
    }

    fn default_public_sphinx_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(Some(id)).join("public_sphinx.pem")
    }

    fn default_location() -> String {
        "unknown".into()
    }
}

impl Default for MixNode {
    fn default() -> Self {
        MixNode {
            id: "".to_string(),
            location: Self::default_location(),
            layer: 0,
            listening_address: format!("0.0.0.0:{}", DEFAULT_LISTENING_PORT)
                .parse()
                .unwrap(),
            announce_address: format!("127.0.0.1:{}", DEFAULT_LISTENING_PORT),
            private_sphinx_key_file: Default::default(),
            public_sphinx_key_file: Default::default(),
            nym_root_directory: Config::default_root_directory(),
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
    // The idea of additional 'directory servers' is to let mixes report their presence
    // and metrics to separate places
    /// Directory server to which the server will be reporting their presence data.
    presence_directory_server: String,

    /// Delay between each subsequent presence data being sent.
    /// The provided value is interpreted as milliseconds.
    presence_sending_delay: u64,

    /// Directory server to which the server will be reporting their metrics data.
    metrics_directory_server: String,

    /// Delay between each subsequent metrics data being sent.
    /// The provided value is interpreted as milliseconds.
    metrics_sending_delay: u64,

    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    /// The provided value is interpreted as milliseconds.
    packet_forwarding_initial_backoff: u64,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    /// The provided value is interpreted as milliseconds.
    packet_forwarding_maximum_backoff: u64,
}

impl Debug {
    fn default_directory_server() -> String {
        #[cfg(feature = "qa")]
        return "https://qa-directory.nymtech.net".to_string();
        #[cfg(feature = "local")]
        return "http://localhost:8080".to_string();

        "https://directory.nymtech.net".to_string()
    }
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            presence_directory_server: Self::default_directory_server(),
            presence_sending_delay: DEFAULT_PRESENCE_SENDING_DELAY,
            metrics_directory_server: Self::default_directory_server(),
            metrics_sending_delay: DEFAULT_METRICS_SENDING_DELAY,
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
        }
    }
}

#[cfg(test)]
mod mixnode_config {
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
