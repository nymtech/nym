//! Configuration for Ephemeris node. It contains mandatory settings for a node to start.
//!
//! Default location for the configuration file is `~/.nym/ephemera/ephemera.toml`.
//! Or relative to a node specific directory `~/.nym/ephemera/<node_name>/ephemera.toml`.

use std::io::Write;
use std::path::PathBuf;

use config::ConfigError;
use log::{error, info};
use nym_config::{DEFAULT_NYM_APIS_DIR, NYM_DIR};
use serde_derive::{Deserialize, Serialize};
use thiserror::Error;

//TODO - validate configuration at load time
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Configuration {
    /// Configuration related to node instance identity
    pub node: NodeConfiguration,
    /// Configuration for libp2p network
    pub libp2p: Libp2pConfiguration,
    /// Configuration for Ephemera embedded database
    pub storage: DatabaseConfiguration,
    /// Configuration for websocket
    pub websocket: WebsocketConfiguration,
    /// Configuration for embedded http API server
    pub http: HttpConfiguration,
    /// Configuration related to block creation
    pub block_manager: BlockManagerConfiguration,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NodeConfiguration {
    /// Node IP shared by all Ephemera services like libp2p, websocket, http.
    /// If separate IP/DNS is needed for each service, it should be configured outside of Ephemera.
    pub ip: String,
    //FIXME: dev only
    /// If private key is stored in configuration as plain text, read it from here.
    /// Private key is mandatory for a node to be able to function in the network.
    /// It is used to signe protocol messages and identify node in the network.
    pub private_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MembershipKind {
    /// Mandatory minimum membership size is defined by threshold of all peers returned by membership provider.
    /// Threshold value is defined the ratio of peers that need to be available.
    /// For example, if the threshold is 0.5, then at least 50% of the peers need to be available.
    Threshold,
    /// Mandatory minimum membership size is all peers who are online.
    AnyOnline,
    /// Mandatory minimum membership size is all peers returned by membership provider.
    AllOnline,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Libp2pConfiguration {
    /// Port to listen on for libp2p internal connections
    pub port: u16,
    /// Gossipsub topic to gossip Ephemera messages between peers. Ephemera listens messages
    /// only from this topic. Invalid topic configuration means that Ephemera is not able to
    /// reach messages from other peers.
    pub ephemera_msg_topic_name: String,
    /// Gossipsub interval to check its mesh health.
    pub heartbeat_interval_sec: u64,
    /// How often Ephemera checks its membership rendezvous endpoint. It's configurable with second granularity.
    /// But in general it's up to the user and depends how rendezvous endpoint is configured.
    ///
    /// Ephemera uses rendezvous endpoint as authority to tell which nodes are authorized to participate.
    /// So it should be configured and implemented in a manner that nodes always have the most up to date and
    /// accurate information.
    pub members_provider_delay_sec: u64,
    /// Defines how the actual membership is decided. See `[ephemera:]` for more details.
    pub membership_kind: MembershipKind,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DatabaseConfiguration {
    /// Path to the RocksDb database directory
    pub rocksdb_path: String,
    /// Path to the SQLite database file
    pub sqlite_path: String,
    /// If to create database if it does not exist
    pub create_if_not_exists: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WebsocketConfiguration {
    /// Port to listen on for WebSocket subscriptions.
    pub port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HttpConfiguration {
    /// Port to listen on for HTTP API requests
    pub port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BlockManagerConfiguration {
    /// By default every node is block producer.
    ///
    /// But in trusted settings it might be useful configure only one or selected nodes to be block producer.
    /// The rest of nodes still participate in the message gossiping and reliable broadcast.
    pub producer: bool,
    /// Interval in seconds between block creation
    /// Blocks are "proposed" at this interval.
    pub creation_interval_sec: u64,
    /// Ephemera creates blocks at fixed interval and doesn't have any consensus algorithm to make progress
    /// if the most recent block fails to go through reliable broadcast.
    /// This flag tells what to do when at next interval previous block is not yet delivered.
    ///
    /// If true, Ephemera will repeat messages from the previous block. Otherwise it will take all messages
    /// from mempool as normally.
    pub repeat_last_block_messages: bool,
}

impl BlockManagerConfiguration {
    pub fn new(producer: bool, creation_interval_sec: u64, repeat_last_block: bool) -> Self {
        BlockManagerConfiguration {
            producer,
            creation_interval_sec,
            repeat_last_block_messages: repeat_last_block,
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    /// This is returned if configuration file exists and user tries to create new one.
    #[error("Configuration file exists: '{0}'")]
    Exists(String),
    /// This is returned if configuration file does not exist and user tries to load it.
    #[error("Configuration file does not exists: '{0}'")]
    NotFound(String),
    #[error("Configuration file does not exist")]
    /// This is returned if IoError happens during configuration file read/write.
    Io(#[from] std::io::Error),
    /// This is returned if configuration file is invalid.
    #[error("Configuration file is invalid: '{0}'")]
    InvalidFormat(String),
    /// This is returned if configuration file path is invalid.
    #[error("Configuration file path is invalid: '{0}'")]
    InvalidPath(String),
    /// Technical error happens during parsing.
    #[error("{}", .0)]
    Other(String),
}

impl From<ConfigError> for Error {
    fn from(err: ConfigError) -> Self {
        match err {
            ConfigError::NotFound(err) => Error::NotFound(err),
            ConfigError::PathParse(err) => {
                Error::InvalidPath(format!("Invalid path to configuration file: {err:?}",))
            }
            ConfigError::FileParse { uri, cause } => {
                Error::InvalidFormat(format!("Invalid configuration file: {uri:?}: {cause:?}",))
            }
            _ => Error::Other(err.to_string()),
        }
    }
}

const EPHEMERA_DIR_NAME: &str = "ephemera";
const EPHEMERA_CONFIG_FILE: &str = "ephemera.toml";

type Result<T> = std::result::Result<T, Error>;

impl Configuration {
    /// Tries to read Ephemera node configuration file (`ephemera.toml`) from the given path.
    ///
    /// # Arguments
    /// * `path` - Path to the configuration file.
    ///
    /// # Errors
    /// Returns an error if the configuration file does not exist or is invalid.
    ///
    /// # Example
    /// ```no_run
    /// use ephemera::configuration::Configuration;
    ///
    /// let config = Configuration::try_load("/ephemera/ephemera.toml");
    /// ```
    pub fn try_load<P: Into<PathBuf>>(path: P) -> Result<Configuration> {
        let buf = path.into();
        log::debug!("Loading configuration from {:?}", buf);
        let config = config::Config::builder()
            .add_source(config::File::from(buf))
            .build()
            .map_err(Error::from)?;

        config.try_deserialize().map_err(Error::from)
    }

    /// Tries to read Ephemera node configuration from default
    /// default Ephemera configuration directory(`~.nym/ephemera`). Full path resolves
    /// as `~/.nym/ephemera/<node_name>/<file>`.
    ///
    /// # Arguments
    /// * `node_name` - Name of the node.
    /// * `file` - Name of the configuration file.
    ///
    /// # Errors
    /// Returns an error if the configuration file does not exist or is invalid.
    pub fn try_load_node_from_home_dir(file: &str) -> Result<Configuration> {
        let file_path = Self::ephemera_node_dir(None)?.join(file);
        Configuration::try_load(file_path)
    }

    /// Tries to read Ephemera node configuration from default Ephemera configuration directory(`~.nym/ephemera`).
    /// Full path resolves as `~/.nym/ephemera/<node_name>/ephemera.toml`.
    ///
    /// # Arguments
    /// * `node_name` - Name of the node.
    ///
    /// # Errors
    /// Returns an error if the configuration file does not exist or is invalid.
    pub fn try_load_from_home_dir() -> Result<Configuration> {
        let file_path = Configuration::ephemera_config_file_home(None)?;
        let config = config::Config::builder()
            .add_source(config::File::from(file_path))
            .build()
            .map_err(Error::from)?;

        config.try_deserialize().map_err(Error::from)
    }

    /// Tries to write(create) Ephemera node configuration file (`ephemera.toml`) relative to default
    /// Ephemera configuration directory(`~.nym/ephemera`). Full path resolves as `~/.nym/ephemera/<node_name>/ephemera.toml`.
    ///
    /// # Arguments
    /// * `id` - Id of the node.
    ///
    /// # Errors
    /// Returns an error if the configuration file already exists.
    ///
    /// # Panics
    /// Panics if the configuration file cannot be written.
    pub fn try_write_home_dir(&self, id: Option<&str>) -> Result<()> {
        let conf_path = Configuration::ephemera_node_dir(id)?;
        if !conf_path.exists() {
            std::fs::create_dir_all(conf_path)?;
        }

        let file_path = Configuration::ephemera_config_file_home(id)?;
        if file_path.exists() {
            return Err(Error::Exists(file_path.to_str().unwrap().to_string()));
        }

        self.write(&file_path)?;
        Ok(())
    }

    /// Tries to write(update) Ephemera node configuration file (`ephemera.toml`) relative to default
    /// Ephemera configuration directory(`~.nym/ephemera`). Full path resolves as `~/.nym/ephemera/<node_name>/ephemera.toml`.
    /// If the file does not exist, update will be refused.
    ///
    /// # Arguments
    /// * `node_name` - Name of the node.
    ///
    /// # Errors
    /// Returns an error if the configuration file does not exist.
    ///
    /// # Panics
    /// Panics if the configuration file cannot be written.
    pub fn try_update_home_dir(&self) -> Result<()> {
        let file_path = Configuration::ephemera_config_file_home(None)?;
        if !file_path.exists() {
            error!(
                "Configuration file does not exist {}",
                file_path.to_str().unwrap()
            );
            return Err(Error::NotFound(file_path.to_str().unwrap().to_string()));
        }
        self.write(&file_path)?;
        Ok(())
    }

    /// Returns node configuration file path relative to default Ephemera configuration directory(`~.nym/ephemera`).
    /// Full path resolves as `~/.nym/ephemera/<id>/ephemera.toml`.
    ///
    /// # Arguments
    /// * `id` - Id of the node.
    ///
    /// # Errors
    /// Returns an error if the configuration file path cannot be resolved.
    pub fn ephemera_config_file_home(id: Option<&str>) -> Result<PathBuf> {
        Ok(Self::ephemera_node_dir(id)?.join(EPHEMERA_CONFIG_FILE))
    }

    /// Returns default Ephemera configuration directory(`~.nym/ephemera`).
    ///
    /// # Errors
    /// Returns an error if the configuration directory cannot be resolved.
    pub fn ephemera_root_dir(id: Option<&str>) -> Result<PathBuf> {
        let id = id.unwrap_or_default();
        dirs::home_dir()
            .map(|home| {
                home.join(NYM_DIR)
                    .join(DEFAULT_NYM_APIS_DIR)
                    .join(id)
                    .join(EPHEMERA_DIR_NAME)
            })
            .ok_or(Error::Other("Could not find home directory".to_string()))
    }

    pub(crate) fn ephemera_node_dir(id: Option<&str>) -> Result<PathBuf> {
        Self::ephemera_root_dir(id)
    }

    fn write(&self, file_path: &PathBuf) -> Result<()> {
        //TODO: use toml or config crate, not both
        let config = toml::to_string(&self).map_err(|e| {
            Error::InvalidFormat(format!("Failed to serialize configuration: {e}",))
        })?;

        let config = format!(
            "#This file is generated by cli and automatically overwritten every time when cli is run\n{config}",
        );

        if file_path.exists() {
            info!("Updating configuration file: '{}'", file_path.display());
        } else {
            info!("Writing configuration to file: '{}'", file_path.display());
        }

        let mut file = std::fs::File::create(file_path)?;
        file.write_all(config.as_bytes())?;

        Ok(())
    }
}
