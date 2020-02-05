use crate::config::template::config_template;
use config::NymConfig;
use log::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time;

mod template;

// 'DEBUG'
// where applicable, the below are defined in milliseconds
const DEFAULT_PRESENCE_SENDING_DELAY: u64 = 3000;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    validator: Validator,

    mix_mining: MixMining,

    tendermint: Tendermint,

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
            .join("validators")
    }

    fn root_directory(&self) -> PathBuf {
        self.validator.nym_root_directory.clone()
    }

    fn config_directory(&self) -> PathBuf {
        self.validator
            .nym_root_directory
            .join(&self.validator.id)
            .join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.validator
            .nym_root_directory
            .join(&self.validator.id)
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

        // calls to any defaults requiring id (see: client, mixnode, provider):

        self.validator.id = id;
        self
    }

    pub fn with_custom_directory<S: Into<String>>(mut self, directory_server: S) -> Self {
        let directory_server_string = directory_server.into();
        self.debug.presence_directory_server = directory_server_string.clone();
        self
    }

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Validator {
    /// ID specifies the human readable ID of this particular validator.
    id: String,

    /// nym_home_directory specifies absolute path to the home nym MixNodes directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,
}

impl Validator {}

impl Default for Validator {
    fn default() -> Self {
        Validator {
            id: "".to_string(),
            nym_root_directory: Config::default_root_directory(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixMining {}

impl Default for MixMining {
    fn default() -> Self {
        MixMining {}
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tendermint {}

impl Default for Tendermint {
    fn default() -> Self {
        Tendermint {}
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
    /// Directory server to which the server will be reporting their presence data.
    presence_directory_server: String,

    /// Delay between each subsequent presence data being sent.
    presence_sending_delay: u64,
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
        }
    }
}

#[cfg(test)]
mod validator_config {
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
