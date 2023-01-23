// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use handlebars::Handlebars;
use network_defaults::mainnet::read_var_if_not_default;
use network_defaults::var_names::CONFIGURED;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::any::type_name;
use std::fmt::Debug;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::str::FromStr;
use std::{fs, io};

pub mod defaults;

pub const CONFIG_DIR: &str = "config";
pub const DATA_DIR: &str = "data";
pub const DB_FILE_NAME: &str = "db.sqlite";

pub trait NymConfig: Default + Serialize + DeserializeOwned {
    fn template() -> &'static str;

    fn config_file_name() -> String {
        "config.toml".to_string()
    }

    fn default_root_directory() -> PathBuf;

    // default, most probable, implementations; can be easily overridden where required
    fn default_config_directory(id: Option<&str>) -> PathBuf {
        if let Some(id) = id {
            Self::default_root_directory().join(id).join(CONFIG_DIR)
        } else {
            Self::default_root_directory().join(CONFIG_DIR)
        }
    }

    fn default_data_directory(id: Option<&str>) -> PathBuf {
        if let Some(id) = id {
            Self::default_root_directory().join(id).join(DATA_DIR)
        } else {
            Self::default_root_directory().join(DATA_DIR)
        }
    }

    fn default_config_file_path(id: Option<&str>) -> PathBuf {
        Self::default_config_directory(id).join(Self::config_file_name())
    }

    // We provide a second set of functions that tries to not panic.

    fn try_default_root_directory() -> Option<PathBuf>;

    fn try_default_config_directory(id: Option<&str>) -> Option<PathBuf> {
        if let Some(id) = id {
            Self::try_default_root_directory().map(|d| d.join(id).join(CONFIG_DIR))
        } else {
            Self::try_default_root_directory().map(|d| d.join(CONFIG_DIR))
        }
    }

    fn try_default_data_directory(id: Option<&str>) -> Option<PathBuf> {
        if let Some(id) = id {
            Self::try_default_root_directory().map(|d| d.join(id).join(DATA_DIR))
        } else {
            Self::try_default_root_directory().map(|d| d.join(DATA_DIR))
        }
    }

    fn try_default_config_file_path(id: Option<&str>) -> Option<PathBuf> {
        Self::try_default_config_directory(id).map(|d| d.join(Self::config_file_name()))
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

        let location = custom_location
            .unwrap_or_else(|| self.config_directory().join(Self::config_file_name()));
        log::info!("Configuration file will be saved to {:?}", location);

        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                fs::write(location.clone(), templated_config)?;
                let mut perms = fs::metadata(location.clone())?.permissions();
                perms.set_mode(0o600);
                fs::set_permissions(location, perms)?;
            } else {
                fs::write(location, templated_config)?;
            }
        }

        Ok(())
    }

    fn load_from_file(id: Option<&str>) -> io::Result<Self> {
        let file = Self::default_config_file_path(id);
        log::trace!("Loading from file: {:#?}", file);
        let config_contents = fs::read_to_string(file)?;

        toml::from_str(&config_contents)
            .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
    }
}

// this function is only used for parsing values from the network defaults and thus the "expect" there are fine
pub fn parse_urls(raw: &str) -> Vec<url::Url> {
    raw.split(',')
        .map(|raw_url| {
            raw_url
                .trim()
                .parse()
                .expect("one of the provided nym api urls is invalid")
        })
        .collect()
}

pub trait OptionalSet {
    fn with_optional<F, T>(self, f: F, val: Option<T>) -> Self
    where
        F: Fn(Self, T) -> Self,
        Self: Sized,
    {
        if let Some(val) = val {
            f(self, val)
        } else {
            self
        }
    }

    fn with_validated_optional<F, T, V, E>(
        self,
        f: F,
        value: Option<T>,
        validate: V,
    ) -> Result<Self, E>
    where
        F: Fn(Self, T) -> Self,
        V: Fn(&T) -> Result<(), E>,
        Self: Sized,
    {
        if let Some(val) = value {
            validate(&val)?;
            Ok(f(self, val))
        } else {
            Ok(self)
        }
    }

    fn with_optional_env<F, T>(self, f: F, val: Option<T>, env_var: &str) -> Self
    where
        F: Fn(Self, T) -> Self,
        T: FromStr,
        <T as FromStr>::Err: Debug,
        Self: Sized,
    {
        if let Some(val) = val {
            return f(self, val);
        } else if std::env::var(CONFIGURED).is_ok() {
            if let Some(raw) = read_var_if_not_default(env_var) {
                return f(
                    self,
                    raw.parse().unwrap_or_else(|err| {
                        panic!(
                            "failed to parse value of {raw} into type {}. the error was {:?}",
                            type_name::<T>(),
                            err
                        )
                    }),
                );
            }
        }
        self
    }

    fn with_optional_custom_env<F, T, G>(
        self,
        f: F,
        val: Option<T>,
        env_var: &str,
        parser: G,
    ) -> Self
    where
        F: Fn(Self, T) -> Self,
        G: Fn(&str) -> T,
        Self: Sized,
    {
        if let Some(val) = val {
            return f(self, val);
        } else if std::env::var(CONFIGURED).is_ok() {
            if let Some(raw) = read_var_if_not_default(env_var) {
                return f(self, parser(&raw));
            }
        }
        self
    }
}

impl<T> OptionalSet for T where T: NymConfig {}
