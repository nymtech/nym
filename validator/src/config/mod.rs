use crate::config::template::config_template;
use config::NymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time;

mod template;

// where applicable, the below are defined in milliseconds

// 'MIXMINING'
const DEFAULT_MIX_MINING_DELAY: u64 = 10_000;
const DEFAULT_MIX_MINING_RESOLUTION_TIMEOUT: u64 = 5_000;

const DEFAULT_NUMBER_OF_MIX_MINING_TEST_PACKETS: u64 = 2;

// 'DEBUG'
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
    fn default_directory_server() -> String {
        #[cfg(feature = "qa")]
        return "https://qa-directory.nymtech.net".to_string();
        #[cfg(feature = "local")]
        return "http://localhost:8080".to_string();

        "https://directory.nymtech.net".to_string()
    }

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
        self.mix_mining.directory_server = directory_server_string;
        self
    }

    pub fn with_location<S: Into<String>>(mut self, location: S) -> Self {
        self.validator.location = location.into();
        self
    }

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }

    #[allow(dead_code)]
    pub fn get_location(&self) -> String {
        self.validator.location.clone()
    }

    pub fn get_mix_mining_directory_server(&self) -> String {
        self.mix_mining.directory_server.clone()
    }

    // dead_code until validator actually sends the presence data
    #[allow(dead_code)]
    pub fn get_presence_directory_server(&self) -> String {
        self.debug.presence_directory_server.clone()
    }

    #[allow(dead_code)]
    pub fn get_presence_sending_delay(&self) -> time::Duration {
        time::Duration::from_millis(self.debug.presence_sending_delay)
    }

    pub fn get_mix_mining_run_delay(&self) -> time::Duration {
        time::Duration::from_millis(self.mix_mining.run_delay)
    }

    pub fn get_mix_mining_resolution_timeout(&self) -> time::Duration {
        time::Duration::from_millis(self.mix_mining.resolution_timeout)
    }

    pub fn get_mix_mining_number_of_test_packets(&self) -> u64 {
        self.mix_mining.number_of_test_packets
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Validator {
    /// ID specifies the human readable ID of this particular validator.
    id: String,

    /// Completely optional value specifying geographical location of this particular node.
    /// Currently it's used entirely for debug purposes, as there are no mechanisms implemented
    /// to verify correctness of the information provided. However, feel free to fill in
    /// this field with as much accuracy as you wish to share.
    location: String,

    /// nym_home_directory specifies absolute path to the home nym MixNodes directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,
}

impl Validator {
    fn default_location() -> String {
        "unknown".into()
    }
}

impl Default for Validator {
    fn default() -> Self {
        Validator {
            id: "".to_string(),
            location: Self::default_location(),
            nym_root_directory: Config::default_root_directory(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixMining {
    /// Directory server from which the validator will obtain initial topology.
    directory_server: String,

    /// The uniform delay every which validator are running their mix-mining procedure.
    /// The provided value is interpreted as milliseconds.
    run_delay: u64,

    /// During the mix-mining process, test packets are sent through various network
    /// paths. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    /// The provided value is interpreted as milliseconds.
    resolution_timeout: u64,

    /// How many packets should be sent through each path during the mix-mining procedure.
    number_of_test_packets: u64,
}

impl Default for MixMining {
    fn default() -> Self {
        MixMining {
            directory_server: Config::default_directory_server(),
            run_delay: DEFAULT_MIX_MINING_DELAY,
            resolution_timeout: DEFAULT_MIX_MINING_RESOLUTION_TIMEOUT,
            number_of_test_packets: DEFAULT_NUMBER_OF_MIX_MINING_TEST_PACKETS,
        }
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

impl Debug {}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            presence_directory_server: Config::default_directory_server(),
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
