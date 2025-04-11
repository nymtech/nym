// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::helpers::store_key;
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::key_rotation::key::SphinxPrivateKey;
use nym_crypto::aes::cipher::crypto_common::rand_core::{CryptoRng, RngCore};
use std::path::{Path, PathBuf};
use tracing::trace;

pub(crate) struct SphinxKeyManager {
    pub(crate) keys: ActiveSphinxKeys,

    primary_key_path: PathBuf,
    secondary_key_path: PathBuf,
}

impl SphinxKeyManager {
    // only called by newly initialised nym-nodes
    pub(crate) fn initialise_new<R, P>(
        rng: &mut R,
        current_rotation_id: u32,
        primary_key_path: P,
        secondary_key_path: P,
    ) -> Result<Self, NymNodeError>
    where
        R: RngCore + CryptoRng,
        P: AsRef<Path>,
    {
        let primary = SphinxPrivateKey::new(rng, current_rotation_id);
        trace!("attempting to store primary x25519 sphinx key");

        let primary_key_path = primary_key_path.as_ref();
        store_key(&primary, primary_key_path, "x25519 sphinx")?;

        Ok(SphinxKeyManager {
            keys: ActiveSphinxKeys::new_fresh(primary),
            primary_key_path: primary_key_path.to_path_buf(),
            secondary_key_path: secondary_key_path.as_ref().to_path_buf(),
        })
    }

    pub(crate) fn try_load<P: AsRef<Path>>(
        primary_key_path: P,
        secondary_key_path: P,
    ) -> Result<Self, NymNodeError> {
        todo!()
    }

    // if upon loading it turns out that the node has been inactive for a long time,
    // immediately rotate keys (but leave 1h grace period for current primary)
}
