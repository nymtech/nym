// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::var_names;
use crate::var_names::{
    DEPRECATED_API_VALIDATOR, DEPRECATED_NYMD_VALIDATOR, NYM_API, NYXD,
    ROOT_ATTESTER_ED25519_BS58_PUBKEY, UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY,
};
use std::path::Path;

fn fix_deprecated_environmental_variables() {
    unsafe {
        // if we're using the outdated environmental variables, set the updated ones to preserve compatibility
        if let Ok(nyxd) = std::env::var(DEPRECATED_NYMD_VALIDATOR)
            && std::env::var(NYXD).is_err()
        {
            std::env::set_var(NYXD, nyxd)
        }
        if let Ok(nym_apis) = std::env::var(DEPRECATED_API_VALIDATOR)
            && std::env::var(NYM_API).is_err()
        {
            std::env::set_var(NYM_API, nym_apis)
        }
        // promote the legacy attester-key name to the canonical one, so a `.env` that still
        // uses the old name is not shadowed by the mainnet-default backfill of the new name
        if let Ok(attester) = std::env::var(UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY)
            && std::env::var(ROOT_ATTESTER_ED25519_BS58_PUBKEY).is_err()
        {
            std::env::set_var(ROOT_ATTESTER_ED25519_BS58_PUBKEY, attester)
        }
    }
}

// Read the variables from the file and log what the corresponding values in the environment are.
fn print_env_vars_with_keys_in_file<P: AsRef<Path> + Copy>(config_env_file: P) {
    let items = dotenvy::from_path_iter(config_env_file)
        .expect("Invalid path to environment configuration file");
    for item in items {
        let (key, val) = item.expect("Invalid item in environment configuration file");
        tracing::debug!("{key}: {val}");
    }
}

pub fn env_configured() -> bool {
    std::env::var(var_names::CONFIGURED).is_ok()
}

pub fn setup_env<P: AsRef<Path>>(config_env_file: Option<P>) {
    match std::env::var(var_names::CONFIGURED) {
        // if the configuration is not already set in the env vars
        Err(std::env::VarError::NotPresent) => {
            if let Some(config_env_file) = &config_env_file {
                tracing::debug!(
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
                tracing::debug!("Loading mainnet defaults");
                crate::mainnet::export_to_env();
            }
        }
        Err(_) => {
            tracing::debug!("Environment variables already set. Using them");
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

#[cfg(test)]
mod tests {
    use super::*;

    // Both sub-scenarios live in one test: they share the same two process-global env vars,
    // so running them sequentially avoids racing with each other.
    #[test]
    fn legacy_attester_env_is_promoted_and_canonical_takes_precedence() {
        // scenario 1: only the legacy name is set -> it is promoted to the canonical name, so a
        // reader keyed on the canonical name resolves to the legacy value
        unsafe {
            std::env::remove_var(ROOT_ATTESTER_ED25519_BS58_PUBKEY);
            std::env::set_var(UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY, "legacy-value");
        }
        fix_deprecated_environmental_variables();
        assert_eq!(
            std::env::var(ROOT_ATTESTER_ED25519_BS58_PUBKEY).as_deref(),
            Ok("legacy-value")
        );

        // scenario 2: both are set -> the canonical value is preserved (not overwritten by the
        // legacy one), so the canonical name takes precedence
        unsafe {
            std::env::set_var(ROOT_ATTESTER_ED25519_BS58_PUBKEY, "canonical-value");
            std::env::set_var(UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY, "legacy-value");
        }
        fix_deprecated_environmental_variables();
        assert_eq!(
            std::env::var(ROOT_ATTESTER_ED25519_BS58_PUBKEY).as_deref(),
            Ok("canonical-value")
        );

        unsafe {
            std::env::remove_var(ROOT_ATTESTER_ED25519_BS58_PUBKEY);
            std::env::remove_var(UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY);
        }
    }
}
