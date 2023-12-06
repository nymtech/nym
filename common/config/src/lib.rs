// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use handlebars::{Handlebars, TemplateRenderError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, io};

pub use helpers::{parse_urls, OptionalSet};
pub use toml::de::Error as TomlDeError;

pub mod defaults;
pub mod helpers;
pub mod legacy_helpers;
pub mod serde_helpers;

pub const NYM_DIR: &str = ".nym";
pub const DEFAULT_NYM_APIS_DIR: &str = "nym-api";
pub const DEFAULT_CONFIG_DIR: &str = "config";
pub const DEFAULT_DATA_DIR: &str = "data";
pub const DEFAULT_CONFIG_FILENAME: &str = "config.toml";

#[cfg(feature = "dirs")]
pub fn must_get_home() -> PathBuf {
    if let Some(home_dir) = std::env::var_os("NYM_HOME_DIR") {
        home_dir.into()
    } else {
        dirs::home_dir().expect("Failed to evaluate $HOME value")
    }
}

#[cfg(feature = "dirs")]
pub fn may_get_home() -> Option<PathBuf> {
    if let Some(home_dir) = std::env::var_os("NYM_HOME_DIR") {
        Some(home_dir.into())
    } else {
        dirs::home_dir()
    }
}

pub trait NymConfigTemplate: Serialize {
    fn template(&self) -> &'static str;

    fn format_to_string(&self) -> String {
        // it is responsibility of whoever is implementing the trait to ensure the template is valid
        Handlebars::new()
            .render_template(self.template(), &self)
            .unwrap()
    }

    fn format_to_writer<W: Write>(&self, writer: W) -> io::Result<()> {
        if let Err(err) = Handlebars::new().render_template_to_write(self.template(), &self, writer)
        {
            match err {
                TemplateRenderError::IOError(err, _) => return Err(err),
                other_err => {
                    // it is responsibility of whoever is implementing the trait to ensure the template is valid
                    panic!("invalid template: {other_err}")
                }
            }
        }

        Ok(())
    }
}

pub fn save_formatted_config_to_file<C, P>(config: &C, path: P) -> io::Result<()>
where
    C: NymConfigTemplate,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    log::info!("trying to save config file to {}", path.display());

    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }

    let file = File::create(path)?;

    // TODO: check for whether any of our configs store anything sensitive
    // and change that to 0o644 instead
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
    }

    config.format_to_writer(file)
}

pub fn deserialize_config_from_toml_str<C>(raw: &str) -> Result<C, TomlDeError>
where
    C: DeserializeOwned,
{
    toml::from_str(raw)
}

pub fn read_config_from_toml_file<C, P>(path: P) -> io::Result<C>
where
    C: DeserializeOwned,
    P: AsRef<Path>,
{
    log::trace!(
        "trying to read config file from {}",
        path.as_ref().display()
    );
    let content = fs::read_to_string(path)?;

    // TODO: should we be preserving original error type instead?
    deserialize_config_from_toml_str(&content)
        .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
}

//
//
//
// pub trait NymConfig: Default + Serialize + DeserializeOwned {
//     fn template(&self) -> &'static str;
//
//     fn config_file_name() -> String {
//         "config.toml".to_string()
//     }
//
//     fn default_root_directory() -> PathBuf;
//
//     // default, most probable, implementations; can be easily overridden where required
//     fn default_config_directory(id: &str) -> PathBuf {
//         Self::default_root_directory()
//             .join(id)
//             .join(DEFAULT_CONFIG_DIR)
//     }
//
//     fn default_data_directory(id: &str) -> PathBuf {
//         Self::default_root_directory()
//             .join(id)
//             .join(DEFAULT_DATA_DIR)
//     }
//
//     fn default_config_file_path(id: &str) -> PathBuf {
//         Self::default_config_directory(id).join(Self::config_file_name())
//     }
//
//     // We provide a second set of functions that tries to not panic.
//
//     fn try_default_root_directory() -> Option<PathBuf>;
//
//     fn try_default_config_directory(id: &str) -> Option<PathBuf> {
//         Self::try_default_root_directory().map(|d| d.join(id).join(DEFAULT_CONFIG_DIR))
//     }
//
//     fn try_default_data_directory(id: &str) -> Option<PathBuf> {
//         Self::try_default_root_directory().map(|d| d.join(id).join(DEFAULT_DATA_DIR))
//     }
//
//     fn try_default_config_file_path(id: &str) -> Option<PathBuf> {
//         Self::try_default_config_directory(id).map(|d| d.join(Self::config_file_name()))
//     }
//
//     fn root_directory(&self) -> PathBuf;
//     fn config_directory(&self) -> PathBuf;
//     fn data_directory(&self) -> PathBuf;
//
//     fn save_to_file(&self, custom_location: Option<PathBuf>) -> io::Result<()> {
//         Ok(())
//         // let reg = Handlebars::new();
//         // // it's whoever is implementing the trait responsibility to make sure you can execute your own template on your data
//         // let templated_config = reg.render_template(Self::template(), self).unwrap();
//         //
//         // // make sure the whole directory structure actually exists
//         // match custom_location.clone() {
//         //     Some(loc) => {
//         //         if let Some(parent_dir) = loc.parent() {
//         //             fs::create_dir_all(parent_dir)
//         //         } else {
//         //             Ok(())
//         //         }
//         //     }
//         //     None => fs::create_dir_all(self.config_directory()),
//         // }?;
//         //
//         // let location = custom_location
//         //     .unwrap_or_else(|| self.config_directory().join(Self::config_file_name()));
//         // log::info!("Configuration file will be saved to {:?}", location);
//         //
//         // cfg_if::cfg_if! {
//         //     if #[cfg(unix)] {
//         //         fs::write(location.clone(), templated_config)?;
//         //         let mut perms = fs::metadata(location.clone())?.permissions();
//         //         perms.set_mode(0o600);
//         //         fs::set_permissions(location, perms)?;
//         //     } else {
//         //         fs::write(location, templated_config)?;
//         //     }
//         // }
//         //
//         // Ok(())
//     }
//
//     fn load_from_file(id: &str) -> io::Result<Self> {
//         let file = Self::default_config_file_path(id);
//         log::trace!("Loading from file: {:#?}", file);
//         let config_contents = fs::read_to_string(file)?;
//
//         toml::from_str(&config_contents)
//             .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
//     }
// }
