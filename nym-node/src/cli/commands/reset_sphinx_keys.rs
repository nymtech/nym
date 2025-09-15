// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::config::persistence::{
    DEFAULT_RD_BLOOMFILTER_FILE_EXT, DEFAULT_RD_BLOOMFILTER_FLUSH_FILE_EXT,
};
use crate::config::upgrade_helpers::try_load_current_config;
use crate::node::helpers::get_current_rotation_id;
use crate::node::key_rotation::manager::SphinxKeyManager;
use nym_crypto::aes::cipher::crypto_common::rand_core::OsRng;
use std::fs;
use std::fs::read_dir;
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, clap::Args)]
pub(crate) struct Args {
    #[clap(flatten)]
    pub(crate) config: ConfigArgs,
}

fn clear_bloomfilters(dir: &PathBuf) -> anyhow::Result<()> {
    let read_dir = read_dir(dir)?;
    for entry in read_dir {
        let entry = entry?;
        let path = entry.path();
        let Some(extension) = path.extension() else {
            continue;
        };
        if extension == DEFAULT_RD_BLOOMFILTER_FILE_EXT
            || extension == DEFAULT_RD_BLOOMFILTER_FLUSH_FILE_EXT
        {
            {
                fs::remove_file(path)?;
            }
        }
    }

    Ok(())
}

pub async fn execute(args: Args) -> anyhow::Result<()> {
    let config = try_load_current_config(args.config.config_path()).await?;

    warn!("RESETTING SPHINX KEYS OF NODE {}", config.id);

    // 1. attempt to retrieve current rotation id
    let current_rotation_id =
        get_current_rotation_id(&config.mixnet.nym_api_urls, &config.mixnet.nyxd_urls).await?;

    // 2. remove all bloomfilters
    info!("clearing old replay protection bloomfilters...");
    clear_bloomfilters(
        &config
            .mixnet
            .replay_protection
            .storage_paths
            .current_bloomfilters_directory,
    )?;

    // 3. remove primary and secondary keys. also a temporary key if it existed
    info!("removing old keys...");
    let tmp_location = config
        .storage_paths
        .keys
        .primary_x25519_sphinx_key_file
        .with_extension("tmp");
    if tmp_location.exists() {
        fs::remove_file(tmp_location)?;
    }

    if config
        .storage_paths
        .keys
        .secondary_x25519_sphinx_key_file
        .exists()
    {
        fs::remove_file(&config.storage_paths.keys.secondary_x25519_sphinx_key_file)?;
    }

    // no need to explicitly remove primary key as the file will be overwritten

    // 4. recreate primary key according to current rotation id
    let mut rng = OsRng;

    info!("generating new key for rotation {current_rotation_id}...");
    let _ = SphinxKeyManager::initialise_new(
        &mut rng,
        current_rotation_id,
        &config.storage_paths.keys.primary_x25519_sphinx_key_file,
        &config.storage_paths.keys.secondary_x25519_sphinx_key_file,
    )?;

    info!("done!");
    Ok(())
}
