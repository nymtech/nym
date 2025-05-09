// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{KeyIOFailure, NymNodeError};
use crate::node::helpers::{load_key, store_key};
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::key_rotation::key::{SphinxPrivateKey, SphinxPublicKey};
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

    // moves the primary key to the secondary file
    // and vice verse, i.e. secondary to the primary
    fn swap_key_files<P: AsRef<Path>>(
        primary_path: P,
        secondary_path: P,
    ) -> Result<(), NymNodeError> {
        let tmp_path = primary_path.as_ref().with_extension("tmp");

        // 1. COPY: primary -> temp
        fs::copy(primary_path.as_ref(), secondary_path.as_ref()).map_err(|err| {
            KeyIOFailure::KeyCopyFailure {
                key: "old x25519 sphinx primary".to_string(),
                source: primary_path.as_ref().to_path_buf(),
                destination: secondary_path.as_ref().to_path_buf(),
                err,
            }
        })?;

        // 2. MOVE: secondary -> primary
        fs::rename(secondary_path.as_ref(), primary_path.as_ref()).map_err(|err| {
            KeyIOFailure::KeyMoveFailure {
                key: "x25519 sphinx secondary".to_string(),
                source: secondary_path.as_ref().to_path_buf(),
                destination: primary_path.as_ref().to_path_buf(),
                err,
            }
        })?;

        // 3. MOVE temp -> secondary
        fs::rename(&tmp_path, secondary_path.as_ref()).map_err(|err| {
            KeyIOFailure::KeyMoveFailure {
                key: "old x25519 sphinx primary".to_string(),
                source: tmp_path.clone(),
                destination: primary_path.as_ref().to_path_buf(),
                err,
            }
        })?;

        // 4. REMOVE: temp
        fs::remove_file(&tmp_path).map_err(|err| KeyIOFailure::KeyRemovalFailure {
            key: "old x25519 sphinx primary (temp location)".to_string(),
            path: tmp_path,
            err,
        })?;
        Ok(())
    }

    pub(crate) fn generate_key_for_new_rotation(
        &self,
        expected_rotation: u32,
    ) -> Result<SphinxPublicKey, NymNodeError> {
        let mut rng = OsRng;
        let new = SphinxPrivateKey::new(&mut rng, expected_rotation);
        let pub_key = (&new).into();
        store_key(
            &new,
            &self.secondary_key_path,
            "x22519 (pre-announced) sphinx",
        )?;

        self.keys.set_secondary(new);
        Ok(pub_key)
    }

    pub(crate) fn rotate_keys(&self) -> Result<(), NymNodeError> {
        if !self.keys.rotate() {
            // we failed to perform the rotation because the secondary key somehow didn't exist
            // we can't do much here, but just generate a brand-new key to rotate into
            let primary = self.keys.primary().rotation_id();
            self.generate_key_for_new_rotation(primary + 1)?;
            self.keys.rotate();
        }
        Self::swap_key_files(&self.primary_key_path, &self.secondary_key_path)
    }

    pub(crate) fn remove_overlap_key(&self) -> Result<(), NymNodeError> {
        self.keys.deactivate_secondary();
        fs::remove_file(&self.secondary_key_path).map_err(|err| {
            KeyIOFailure::KeyRemovalFailure {
                key: "old x25519 sphinx secondary".to_string(),
                path: self.secondary_key_path.clone(),
                err,
            }
        })?;
        Ok(())
    }

    pub(crate) fn try_load_or_regenerate<P: AsRef<Path>>(
        current_rotation_id: u32,
        primary_key_path: P,
        secondary_key_path: P,
    ) -> Result<Self, NymNodeError> {
        todo!("check if primary and secondary are correct - we might have crashed during the file swap");

        // // check the temporary location in case we crashed in the middle of rotating the key
        // let tmp_location = primary_key_path.as_ref().with_extension("tmp");
        // if tmp_location.exists() {
        //     warn!("we seem to have crashed in the middle of rotating the sphinx key");
        //     // if temporary key exists, it means it has never overwritten the primary;
        //     // secondary key might or might have not gotten overwritten, but that doesn't matter,
        //     // we can do it again
        //     Self:swape_key_files(primary_key_path.as_ref(), secondary_key_path.as_ref())?;
        // }
        //
        // // primary key should always be present
        // let primary: SphinxPrivateKey =
        //     load_key(primary_key_path.as_ref(), "x25519 sphinx primary")?;
        //
        // // if upon loading it turns out that the node has been inactive for a long time,
        // // immediately rotate keys (but leave 1h grace period for current primary, i.e. set it as secondary)
        // if primary.rotation_id() != current_rotation_id {
        //     warn!("this node has been inactive for more than a key rotation duration. the current primary key was generated for rotation {} while the current rotation is {current_rotation_id}. new key will be generated now.", primary.rotation_id());
        //     let mut this = SphinxKeyManager {
        //         keys: ActiveSphinxKeys::new_loaded(primary, None),
        //         primary_key_path: primary_key_path.as_ref().to_path_buf(),
        //         secondary_key_path: secondary_key_path.as_ref().to_path_buf(),
        //     };
        //     this.rotate_keys(current_rotation_id)?;
        //     return Ok(this);
        // }
        //
        // // secondary key **might** be present
        // let secondary_path = secondary_key_path.as_ref();
        //
        // let secondary = if secondary_path.exists() {
        //     Some(load_key::<SphinxPrivateKey, _>(
        //         secondary_key_path.as_ref(),
        //         "x25519 sphinx secondary",
        //     )?)
        // } else {
        //     None
        // };
        //
        // Ok(SphinxKeyManager {
        //     keys: ActiveSphinxKeys::new_loaded(primary, secondary),
        //     primary_key_path: primary_key_path.as_ref().to_path_buf(),
        //     secondary_key_path: secondary_key_path.as_ref().to_path_buf(),
        // })
    }
}
