// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use handlebars::Handlebars;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;
use std::{fs, io};

pub mod helpers;

pub use helpers::{deserialize_duration, deserialize_validators};

pub trait NymConfig: Default + Serialize + DeserializeOwned {
    fn template() -> &'static str;

    fn config_file_name() -> String {
        "config.toml".to_string()
    }

    fn default_root_directory() -> PathBuf;

    // default, most probable, implementations; can be easily overridden where required
    fn default_config_directory(id: Option<&str>) -> PathBuf {
        if let Some(id) = id {
            Self::default_root_directory().join(id).join("config")
        } else {
            Self::default_root_directory().join("config")
        }
    }

    fn default_data_directory(id: Option<&str>) -> PathBuf {
        if let Some(id) = id {
            Self::default_root_directory().join(id).join("data")
        } else {
            Self::default_root_directory().join("data")
        }
    }

    fn default_config_file_path(id: Option<&str>) -> PathBuf {
        Self::default_config_directory(id).join(Self::config_file_name())
    }

    fn root_directory(&self) -> PathBuf;
    fn config_directory(&self) -> PathBuf;
    fn data_directory(&self) -> PathBuf;

    fn save_to_file(&self, custom_location: Option<PathBuf>) -> io::Result<()> {
        let reg = Handlebars::new();
        // it's whoever is implementing the trait responsibility to make sure you can execute your own template on your data
        let templated_config = reg.render_template(Self::template(), self).unwrap();

        // make sure the whole directory structure actually exists
        match custom_location.clone() {
            Some(loc) => {
                if let Some(parent_dir) = loc.parent() {
                    fs::create_dir_all(parent_dir)
                } else {
                    Ok(())
                }
            }
            None => fs::create_dir_all(self.config_directory()),
        }?;

        fs::write(
            custom_location
                .unwrap_or_else(|| self.config_directory().join(Self::config_file_name())),
            templated_config,
        )
    }

    fn load_from_file(id: Option<&str>) -> io::Result<Self> {
        let config_contents = fs::read_to_string(Self::default_config_file_path(id))?;

        toml::from_str(&config_contents)
            .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
    }
}
