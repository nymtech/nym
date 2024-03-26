// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::default_config_filepath;
use crate::config::old_config_v1_1_13::OldConfigV1_1_13;
use crate::config::old_config_v1_1_20::ConfigV1_1_20;
use crate::config::old_config_v1_1_20_2::ConfigV1_1_20_2;
use crate::config::old_config_v1_1_33::ConfigV1_1_33;
use crate::error::NetworkRequesterError;
use log::{info, trace};
use nym_client_core::client::base_client::storage::migration_helpers::v1_1_33;
use std::path::Path;

async fn try_upgrade_v1_1_13_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.13 config");
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.13 (which is incompatible with the next step, i.e. 1.1.19)
    let Ok(old_config) = OldConfigV1_1_13::load_from_filepath(config_path) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.13 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_20 = old_config.into();
    let updated_step2: ConfigV1_1_20_2 = updated_step1.into();
    let (updated_step3, gateway_config) = updated_step2.upgrade()?;
    let old_paths = updated_step3.storage_paths.clone();
    let updated = updated_step3.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_20_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.20 config");
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.20 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20::load_from_filepath(config_path) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };

    info!("It seems the client is using <= v1.1.20 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_20_2 = old_config.into();
    let (updated_step2, gateway_config) = updated_step1.upgrade()?;
    let old_paths = updated_step2.storage_paths.clone();
    let updated = updated_step2.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_20_2_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.20_2 config");

    // explicitly load it as v1.1.20_2 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20_2::read_from_toml_file(config_path) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20_2 config template.");
    info!("It is going to get updated to the current specification.");

    let (updated_step1, gateway_config) = old_config.upgrade()?;
    let old_paths = updated_step1.storage_paths.clone();
    let updated = updated_step1.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_33_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    // explicitly load it as v1.1.33 (which is incompatible with the current one, i.e. +1.1.34)
    let Ok(old_config) = ConfigV1_1_33::read_from_toml_file(config_path) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.33 config template.");
    info!("It is going to get updated to the current specification.");

    let old_paths = old_config.storage_paths.clone();
    let updated = old_config.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        None,
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

pub async fn try_upgrade_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<(), NetworkRequesterError> {
    trace!("Attempting to upgrade config");
    if try_upgrade_v1_1_13_config(config_path.as_ref()).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_config(config_path.as_ref()).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_2_config(config_path.as_ref()).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_33_config(config_path).await? {
        return Ok(());
    }

    Ok(())
}

pub async fn try_upgrade_config_by_id(id: &str) -> Result<(), NetworkRequesterError> {
    try_upgrade_config(default_config_filepath(id)).await
}
