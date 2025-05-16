// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// removed in 1.1.19/1.1.20
pub mod nym_config {
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use std::path::{Path, PathBuf};
    use std::{fs, io};

    pub const CONFIG_DIR: &str = "config";
    pub const DATA_DIR: &str = "data";

    // no need for anything to do with saving.
    pub trait MigrationNymConfig: Serialize + DeserializeOwned {
        fn config_file_name() -> String {
            "config.toml".to_string()
        }

        fn default_root_directory() -> PathBuf;

        fn default_data_directory(id: &str) -> PathBuf {
            Self::default_data_directory_with_root(Self::default_root_directory(), id)
        }

        fn default_data_directory_with_root<P: AsRef<Path>>(root: P, id: &str) -> PathBuf {
            root.as_ref().join(id).join(DATA_DIR)
        }

        fn default_config_directory(id: &str) -> PathBuf {
            Self::default_config_directory_with_root(Self::default_root_directory(), id)
        }

        fn default_config_directory_with_root<P: AsRef<Path>>(root: P, id: &str) -> PathBuf {
            root.as_ref().join(id).join(CONFIG_DIR)
        }

        fn default_config_file_path(id: &str) -> PathBuf {
            Self::default_config_directory(id).join(Self::config_file_name())
        }

        fn load_from_file(id: &str) -> io::Result<Self> {
            let file = Self::default_config_file_path(id);
            Self::load_from_filepath(file)
        }

        fn load_from_filepath<P: AsRef<Path>>(filepath: P) -> io::Result<Self> {
            log::trace!("Loading from file: {:#?}", filepath.as_ref().to_owned());
            let config_contents = fs::read_to_string(filepath)?;

            toml::from_str(&config_contents).map_err(io::Error::other)
        }
    }
}
