use crate::config::template::config_template;
use config::NymConfig;
use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;

mod template;

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
    pub fn new<S: Into<String>>(id: S) -> Self {
        Config::default().with_id(id)
    }

    // builder methods
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        let id = id.into();
        if self
            .mixnode
            .private_identity_key_file
            .as_os_str()
            .is_empty()
        {
            self.mixnode.private_identity_key_file =
                self::MixNode::default_private_identity_key_file(&id);
        }
        if self.mixnode.public_identity_key_file.as_os_str().is_empty() {
            self.mixnode.public_identity_key_file =
                self::MixNode::default_public_identity_key_file(&id);
        }
        self.mixnode.id = id;
        self
    }

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }

    pub fn get_private_identity_key_file(&self) -> PathBuf {
        self.mixnode.private_identity_key_file.clone()
    }

    pub fn get_public_identity_key_file(&self) -> PathBuf {
        self.mixnode.public_identity_key_file.clone()
    }

    pub fn get_directory_server(&self) -> String {
        self.mixnode.directory_server.clone()
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixNode {
    /// ID specifies the human readable ID of this particular mixnode.
    id: String,

    /// URL to the directory server.
    directory_server: String,

    /// Path to file containing private identity key.
    private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    public_identity_key_file: PathBuf,

    /// nym_home_directory specifies absolute path to the home nym MixNodes directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,
}

impl MixNode {
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

impl Default for MixNode {
    fn default() -> Self {
        MixNode {
            id: "".to_string(),
            directory_server: Self::default_directory_server(),
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
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
pub struct Debug {}

impl Default for Debug {
    fn default() -> Self {
        Debug {}
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
