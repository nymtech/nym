// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::default_config_filepath;
use crate::config::old::v5::ConfigV5;
use crate::config::old_config_v1_1_13::OldConfigV1;
use crate::config::old_config_v1_1_20::ConfigV2;
use crate::config::old_config_v1_1_20_2::ConfigV3;
use crate::config::old_config_v1_1_33::ConfigV4;
use crate::config::Config;
use crate::error::NetworkRequesterError;
use log::{info, trace};
use nym_client_core::cli_helpers::CliClientConfig;
use nym_client_core::client::base_client::storage::migration_helpers::v1_1_33;
use std::path::Path;

async fn try_upgrade_v1_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.13 config");
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.13 (which is incompatible with the next step, i.e. 1.1.19)
    let Ok(old_config) = OldConfigV1::load_from_filepath(config_path.as_ref()) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.13 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV2 = old_config.into();
    let updated_step2: ConfigV3 = updated_step1.into();
    let (updated_step3, gateway_config) = updated_step2.upgrade()?;
    let old_paths = updated_step3.storage_paths.clone();
    let updated_step4 = updated_step3.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated_step4.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    let updated: Config = updated_step4.into();

    updated.save_to(config_path)?;
    Ok(true)
}

async fn try_upgrade_v2_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.20 config");
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.20 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV2::load_from_filepath(config_path.as_ref()) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };

    info!("It seems the client is using <= v1.1.20 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV3 = old_config.into();
    let (updated_step2, gateway_config) = updated_step1.upgrade()?;
    let old_paths = updated_step2.storage_paths.clone();
    let updated_step3 = updated_step2.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated_step3.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    let updated: Config = updated_step3.into();

    updated.save_to(config_path)?;
    Ok(true)
}

async fn try_upgrade_v3_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.20_2 config");

    // explicitly load it as v1.1.20_2 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV3::read_from_toml_file(config_path.as_ref()) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20_2 config template.");
    info!("It is going to get updated to the current specification.");

    let (updated_step1, gateway_config) = old_config.upgrade()?;
    let old_paths = updated_step1.storage_paths.clone();
    let updated_step2 = updated_step1.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated_step2.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    let updated: Config = updated_step2.into();

    updated.save_to(config_path)?;
    Ok(true)
}

async fn try_upgrade_v4_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.33 config");

    // explicitly load it as v1.1.33 (which is incompatible with the current one, i.e. +1.1.34)
    let Ok(old_config) = ConfigV4::read_from_toml_file(config_path.as_ref()) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.33 config template.");
    info!("It is going to get updated to the current specification.");

    let old_paths = old_config.storage_paths.clone();
    let updated_step1 = old_config.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated_step1.storage_paths.common_paths,
        None,
    )
    .await?;

    let updated: Config = updated_step1.into();

    updated.save_to(config_path)?;
    Ok(true)
}

async fn try_upgrade_v5_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<bool, NetworkRequesterError> {
    // explicitly load it as v5 (which is incompatible with the current one)
    let Ok(old_config) = ConfigV5::read_from_toml_file(config_path.as_ref()) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v5 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    updated.save_to(config_path)?;

    Ok(true)
}

pub async fn try_upgrade_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<(), NetworkRequesterError> {
    trace!("Attempting to upgrade config");
    if try_upgrade_v1_config(config_path.as_ref()).await? {
        return Ok(());
    }
    if try_upgrade_v2_config(config_path.as_ref()).await? {
        return Ok(());
    }
    if try_upgrade_v3_config(config_path.as_ref()).await? {
        return Ok(());
    }
    if try_upgrade_v4_config(config_path.as_ref()).await? {
        return Ok(());
    }
    if try_upgrade_v5_config(config_path).await? {
        return Ok(());
    }

    Ok(())
}

pub async fn try_upgrade_config_by_id(id: &str) -> Result<(), NetworkRequesterError> {
    try_upgrade_config(default_config_filepath(id)).await
}
