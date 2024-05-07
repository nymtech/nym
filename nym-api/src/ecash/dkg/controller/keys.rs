// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::client::Client;
use crate::ecash::keys::KeyPairWithEpoch;
use crate::support::{config, nyxd};
use anyhow::{anyhow, bail, Context};
use nym_coconut_dkg_common::types::{EpochId, EpochState};
use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
use rand::{CryptoRng, RngCore};
use std::path::Path;
use thiserror::__private::AsDisplay;

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
    )
    .context("DKG BTE keypair store failure")
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
) -> anyhow::Result<Option<KeyPairWithEpoch>> {
    if !config.storage_paths.coconut_key_path.exists() {
        return Ok(None);
    }
    nym_pemstore::load_key(&config.storage_paths.coconut_key_path)
        .context("coconut key load failure")
        .map(Some)
}

// the keys can be considered valid if they were generated for the current dkg epoch
// and we're either in the "in progress" or "key finalization" states of the DKG
pub(crate) async fn can_validate_coconut_keys(
    nyxd_client: &nyxd::Client,
    issued_for: EpochId,
) -> anyhow::Result<bool> {
    // validate the keys if they were generated for the current dkg epoch
    // and we're either in the "in progress" or "key finalization" states of the DKG
    let current_dkg_epoch = nyxd_client.get_current_epoch().await?;
    if issued_for != current_dkg_epoch.epoch_id {
        warn!("managed to load coconut keys, but they were generated for epoch {issued_for}. The current epoch is {}. the keys won't be used for credential issuance", current_dkg_epoch.epoch_id);
        Ok(false)
    } else if !matches!(
        current_dkg_epoch.state,
        EpochState::InProgress | EpochState::VerificationKeyFinalization { .. }
    ) {
        warn!("managed to load coconut keys, but the current DKG epoch is at {}. the keys won't (yet) be used for credential issuance", current_dkg_epoch.state);
        Ok(false)
    } else {
        Ok(true)
    }
}

pub(crate) fn persist_coconut_keypair<P: AsRef<Path>>(
    keys: &KeyPairWithEpoch,
    store_path: P,
) -> anyhow::Result<()> {
    nym_pemstore::store_key(keys, store_path).context("coconut key store failure")
}

pub(crate) fn archive_coconut_keypair<P: AsRef<Path>>(
    store_path: P,
    epoch_id: EpochId,
) -> anyhow::Result<()> {
    let store_path = store_path.as_ref();
    if !store_path.exists() {
        bail!("coconut key does not exist at {}", store_path.as_display())
    }

    let dir = store_path
        .parent()
        .ok_or(anyhow!("the coconut key does not have a valid parent"))?;
    let filename = store_path
        .file_name()
        .ok_or(anyhow!("the coconut key does not have a valid filename"))?
        .to_str()
        .ok_or(anyhow!("the coconut key filename is not valid UTF8"))?;
    let archive_path = dir.join(format!("epoch-{epoch_id}-{filename}.archived"));
    std::fs::rename(store_path, archive_path)?;

    Ok(())
}
