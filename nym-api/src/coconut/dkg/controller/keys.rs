// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::config;
use anyhow::Context;
use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
use rand::{CryptoRng, RngCore};

pub(crate) fn init_bte_keypair<R: RngCore + CryptoRng>(
    rng: &mut R,
    config: &config::CoconutSigner,
) -> anyhow::Result<()> {
    let dkg_params = nym_dkg::bte::setup();
    let kp = DkgKeyPair::new(&dkg_params, rng);
    nym_pemstore::store_keypair(
        &kp,
        &nym_pemstore::KeyPairPath::new(
            &config.storage_paths.decryption_key_path,
            &config.storage_paths.public_key_with_proof_path,
        ),
    )?;
    Ok(())
}

pub(crate) fn load_bte_keypair(config: &config::CoconutSigner) -> anyhow::Result<DkgKeyPair> {
    nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
        &config.storage_paths.decryption_key_path,
        &config.storage_paths.public_key_with_proof_path,
    ))
    .context("bte keypair load failure")
}

pub(crate) fn load_coconut_keypair_if_exists(
    config: &config::CoconutSigner,
) -> anyhow::Result<Option<nym_coconut_interface::KeyPair>> {
    if !config.storage_paths.secret_key_path.exists() {
        return Ok(None);
    }
    nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
        &config.storage_paths.secret_key_path,
        &config.storage_paths.verification_key_path,
    ))
    .context("coconut keypair load failure")
    .map(Some)
}
