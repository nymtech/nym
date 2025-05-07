// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{KeyIOFailure, NymNodeError};
use crate::node::helpers::{load_key, store_key};
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::key_rotation::key::SphinxPrivateKey;
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{trace, warn};

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

    fn replace_key_files<P: AsRef<Path>>(
        primary_path: P,
        secondary_path: P,
    ) -> Result<(), NymNodeError> {
        let tmp_path = primary_path.as_ref().with_extension("tmp");

        fs::rename(primary_path.as_ref(), secondary_path.as_ref()).map_err(|err| {
            KeyIOFailure::KeyMoveFailure {
                key: "old x25519 sphinx primary".to_string(),
                source: primary_path.as_ref().to_path_buf(),
                destination: secondary_path.as_ref().to_path_buf(),
                err,
            }
        })?;

        fs::rename(&tmp_path, primary_path.as_ref()).map_err(|err| {
            KeyIOFailure::KeyMoveFailure {
                key: "new x25519 sphinx primary".to_string(),
                source: tmp_path,
                destination: primary_path.as_ref().to_path_buf(),
                err,
            }
        })?;
        Ok(())
    }

    // 1. generate new key
    // 2. save it in a temp file
    // 3. move primary key file to the secondary file location (thus losing the secondary)
    // 4. move the temp file to the primary file location
    // 5. set primary as the secondary
    // 6. set new key as the primary
    // 7. (outside this method) broadcast update to nym-apis
    pub(crate) fn rotate_keys(&mut self, current_rotation_id: u32) -> Result<(), NymNodeError> {
        let mut rng = OsRng;
        let new_primary = SphinxPrivateKey::new(&mut rng, current_rotation_id);

        let tmp_path = self.primary_key_path.with_extension("tmp");
        store_key(&new_primary, &tmp_path, "x25519 sphinx")?;

        Self::replace_key_files(&self.primary_key_path, &self.secondary_key_path)?;

        self.keys.rotate(new_primary);
        Ok(())
    }

    pub(crate) fn try_load_or_regenerate<P: AsRef<Path>>(
        current_rotation_id: u32,
        primary_key_path: P,
        secondary_key_path: P,
    ) -> Result<Self, NymNodeError> {
        // check the temporary location in case we crashed in the middle of rotating the key
        let tmp_location = primary_key_path.as_ref().with_extension("tmp");
        if tmp_location.exists() {
            warn!("we seem to have crashed in the middle of rotating the sphinx key");
            // if temporary key exists, it means it has never overwritten the primary;
            // secondary key might or might have not gotten overwritten, but that doesn't matter,
            // we can do it again
            Self::replace_key_files(primary_key_path.as_ref(), secondary_key_path.as_ref())?;
        }

        // primary key should always be present
        let primary: SphinxPrivateKey =
            load_key(primary_key_path.as_ref(), "x25519 sphinx primary")?;

        // if upon loading it turns out that the node has been inactive for a long time,
        // immediately rotate keys (but leave 1h grace period for current primary, i.e. set it as secondary)
        if primary.rotation_id() != current_rotation_id {
            warn!("this node has been inactive for more than a key rotation duration. the current primary key was generated for rotation {} while the current rotation is {current_rotation_id}. new key will be generated now.", primary.rotation_id());
            let mut this = SphinxKeyManager {
                keys: ActiveSphinxKeys::new_loaded(primary, None),
                primary_key_path: primary_key_path.as_ref().to_path_buf(),
                secondary_key_path: secondary_key_path.as_ref().to_path_buf(),
            };
            this.rotate_keys(current_rotation_id)?;
            return Ok(this);
        }

        // secondary key **might** be present
        let secondary_path = secondary_key_path.as_ref();

        let secondary = if secondary_path.exists() {
            Some(load_key::<SphinxPrivateKey, _>(
                secondary_key_path.as_ref(),
                "x25519 sphinx secondary",
            )?)
        } else {
            None
        };

        Ok(SphinxKeyManager {
            keys: ActiveSphinxKeys::new_loaded(primary, secondary),
            primary_key_path: primary_key_path.as_ref().to_path_buf(),
            secondary_key_path: secondary_key_path.as_ref().to_path_buf(),
        })
    }
}
