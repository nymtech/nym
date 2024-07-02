// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg::controller::keys::init_bte_keypair;
use crate::support::config;
use crate::support::config::{
    default_config_directory, default_data_directory, upgrade_helpers, Config,
};
use anyhow::{Context, Result};
use nym_crypto::asymmetric::identity;
use rand::rngs::OsRng;
use std::{fs, io};

// TODO: once we upgrade ed25519 library, we could use the same rand library and use proper
// <R: RngCore + CryptoRng> bound
fn init_identity_keys(config: &config::NymApiPaths) -> Result<()> {
    let keypaths = nym_pemstore::KeyPairPath::new(
        &config.private_identity_key_file,
        &config.public_identity_key_file,
    );

    let mut rng = OsRng;
    let keypair = identity::KeyPair::new(&mut rng);
    nym_pemstore::store_keypair(&keypair, &keypaths)
        .context("failed to store identity keys of the nym api")?;
    Ok(())
}

fn init_paths(id: &str) -> io::Result<()> {
    fs::create_dir_all(default_data_directory(id))?;
    fs::create_dir_all(default_config_directory(id))
}

pub(crate) fn initialise_new(id: &str) -> Result<Config> {
    let config = Config::new(id);

    // create base storage paths
    init_paths(id)?;

    // create identity keys
    init_identity_keys(&config.base.storage_paths)?;

    // create DKG BTE keys
    let mut rng = OsRng;
    init_bte_keypair(&mut rng, &config.coconut_signer)?;
    Ok(config)
}

pub(crate) fn try_load_current_config(id: &str) -> Result<Config> {
    upgrade_helpers::try_upgrade_config(id)?;
    Config::read_from_default_path(id).context(
        "failed to load config.toml from the default path - are you sure you run `init` before?",
    )
}
