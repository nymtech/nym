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

use handlebars::Handlebars;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;
use std::{fs, io};

pub trait NymConfig: Default + Serialize + DeserializeOwned {
    fn template() -> &'static str;

    fn config_file_name() -> String {
        "config.toml".to_string()
    }

    fn default_root_directory() -> PathBuf;

    // default, most probable, implementations; can be easily overridden where required
    fn default_config_directory(id: &str) -> PathBuf {
        Self::default_root_directory().join(id).join("config")
    }

    fn default_data_directory(id: &str) -> PathBuf {
        Self::default_root_directory().join(id).join("data")
    }

    fn default_config_file_path(id: &str) -> PathBuf {
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

    fn load_from_file(custom_location: Option<PathBuf>, id: &str) -> io::Result<Self> {
        let config_contents = fs::read_to_string(
            custom_location.unwrap_or_else(|| Self::default_config_file_path(id)),
        )?;

        toml::from_str(&config_contents)
            .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
    }
}
