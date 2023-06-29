// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        old_config_v1_1_13::OldConfigV1_1_13, old_config_v1_1_20::ConfigV1_1_20,
        old_config_v1_1_20_2::ConfigV1_1_20_2, Config,
    },
    error::{BackendError, Result},
};
use log::info;
use nym_client_core::{
    client::{
        base_client::storage::gateway_details::{OnDiskGatewayDetails, PersistedGatewayDetails},
        key_manager::persistence::OnDiskKeys,
    },
    config::GatewayEndpointConfig,
    error::ClientCoreError,
};

fn persist_gateway_details(config: &Config, details: GatewayEndpointConfig) -> Result<()> {
    let details_store =
        OnDiskGatewayDetails::new(&config.storage_paths.common_paths.gateway_details);
    let keys_store = OnDiskKeys::new(config.storage_paths.common_paths.keys.clone());
    let shared_keys = keys_store.ephemeral_load_gateway_keys().map_err(|source| {
        BackendError::ClientCoreError {
            source: ClientCoreError::KeyStoreError {
                source: Box::new(source),
            },
        }
    })?;
    let persisted_details = PersistedGatewayDetails::new(details, &shared_keys);
    details_store
        .store_to_disk(&persisted_details)
        .map_err(|source| BackendError::ClientCoreError {
            source: ClientCoreError::GatewayDetailsStoreError {
                source: Box::new(source),
            },
        })
}
fn try_upgrade_v1_1_13_config(id: &str) -> Result<bool> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.13 (which is incompatible with the next step, i.e. 1.1.19)
    let Ok(old_config) = OldConfigV1_1_13::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.13 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_20 = old_config.into();
    let updated_step2: ConfigV1_1_20_2 = updated_step1.into();
    let (updated, gateway_config) = updated_step2.upgrade();
    persist_gateway_details(&updated, gateway_config)?;

    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_v1_1_20_config(id: &str) -> Result<bool> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.20 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_20_2 = old_config.into();
    let (updated, gateway_config) = updated_step1.upgrade();
    persist_gateway_details(&updated, gateway_config)?;

    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_v1_1_20_2_config(id: &str) -> Result<bool> {
    // explicitly load it as v1.1.20_2 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20_2::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20_2 config template.");
    info!("It is going to get updated to the current specification.");

    let (updated, gateway_config) = old_config.upgrade();
    persist_gateway_details(&updated, gateway_config)?;

    updated.save_to_default_location()?;
    Ok(true)
}

pub fn try_upgrade_config(id: &str) -> Result<()> {
    if try_upgrade_v1_1_13_config(id)? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_config(id)? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_2_config(id)? {
        return Ok(());
    }

    Ok(())
}
