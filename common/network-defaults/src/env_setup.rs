// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::var_names;
use crate::var_names::{DEPRECATED_API_VALIDATOR, DEPRECATED_NYMD_VALIDATOR, NYM_API, NYXD};
use std::path::Path;

fn fix_deprecated_environmental_variables() {
    // if we're using the outdated environmental variables, set the updated ones to preserve compatibility
    if let Ok(nyxd) = std::env::var(DEPRECATED_NYMD_VALIDATOR) {
        if std::env::var(NYXD).is_err() {
            std::env::set_var(NYXD, nyxd)
        }
    }
    if let Ok(nym_apis) = std::env::var(DEPRECATED_API_VALIDATOR) {
        if std::env::var(NYM_API).is_err() {
            std::env::set_var(NYM_API, nym_apis)
        }
    }
}

// Read the variables from the file and log what the corresponding values in the environment are.
fn print_env_vars_with_keys_in_file<P: AsRef<Path> + Copy>(config_env_file: P) {
    let items = dotenvy::from_path_iter(config_env_file)
        .expect("Invalid path to environment configuration file");
    for item in items {
        let (key, val) = item.expect("Invalid item in environment configuration file");
        log::debug!("{}: {}", key, val);
    }
}

pub fn setup_env<P: AsRef<Path>>(config_env_file: Option<P>) {
    match std::env::var(var_names::CONFIGURED) {
        // if the configuration is not already set in the env vars
        Err(std::env::VarError::NotPresent) => {
            if let Some(config_env_file) = &config_env_file {
                log::debug!(
                    "Loading environment variables from {:?}",
                    config_env_file.as_ref()
                );
                dotenvy::from_path(config_env_file)
                    .expect("Invalid path to environment configuration file");
                fix_deprecated_environmental_variables();
            } else {
                // if nothing is set, the use mainnet defaults
                // if the user has not set `CONFIGURED`, then even if they set any of the env variables,
                // overwrite them
                log::debug!("Loading mainnet defaults");
                crate::mainnet::export_to_env();
            }
        }
        Err(_) => {
            log::debug!("Environment variables already set. Using them");
            crate::mainnet::export_to_env()
        }
        _ => {
            fix_deprecated_environmental_variables();
        }
    }

    // if we haven't explicitly defined any of the constants, fallback to defaults
    crate::mainnet::export_to_env_if_not_set();

    if let Some(config_env_file) = &config_env_file {
        print_env_vars_with_keys_in_file(config_env_file);
    }
}
