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
use tracing::{debug, warn};

pub(crate) fn init_bte_keypair<R: RngCore + CryptoRng>(
    rng: &mut R,
    config: &config::EcashSigner,
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

pub(crate) fn load_bte_keypair(config: &config::EcashSigner) -> anyhow::Result<DkgKeyPair> {
    nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
        &config.storage_paths.decryption_key_path,
        &config.storage_paths.public_key_with_proof_path,
    ))
    .context("bte keypair load failure")
}

const ARCHIVED_KEY_PREFIX: &str = "epoch-";
const ARCHIVED_KEY_SUFFIX: &str = ".archived";

/// Split the path of the live ecash key into the pieces the archive naming needs.
fn key_path_parts(store_path: &Path) -> anyhow::Result<(&Path, &str)> {
    let dir = store_path
        .parent()
        .ok_or(anyhow!("the ecash key does not have a valid parent"))?;
    let filename = store_path
        .file_name()
        .ok_or(anyhow!("the ecash key does not have a valid filename"))?
        .to_str()
        .ok_or(anyhow!("the ecash key filename is not valid UTF8"))?;

    Ok((dir, filename))
}

/// The name the ecash key takes once `epoch_id` is no longer the epoch we sign for.
fn archived_key_filename(live_filename: &str, epoch_id: EpochId) -> String {
    format!("{ARCHIVED_KEY_PREFIX}{epoch_id}-{live_filename}{ARCHIVED_KEY_SUFFIX}")
}

/// The epoch `filename` encodes, if it is an archive of `live_filename` at all.
fn archived_key_epoch(live_filename: &str, filename: &str) -> Option<EpochId> {
    filename
        .strip_prefix(ARCHIVED_KEY_PREFIX)?
        .strip_suffix(ARCHIVED_KEY_SUFFIX)?
        .strip_suffix(live_filename)?
        .strip_suffix('-')?
        .parse()
        .ok()
}

pub(crate) fn load_ecash_keypair_if_exists(
    config: &config::EcashSigner,
) -> anyhow::Result<Option<KeyPairWithEpoch>> {
    let storage_path = &config.storage_paths.ecash_key_path;
    debug!(
        "attempting to ecash keypair from {}",
        storage_path.display()
    );
    if !config.storage_paths.ecash_key_path.exists() {
        debug!("the provided filepath doesn't exist - the key won't be loaded");
        return Ok(None);
    }

    let ecash_key =
        nym_pemstore::load_key::<KeyPairWithEpoch, _>(&config.storage_paths.ecash_key_path)
            .context("failed to load ecash key")?;
    Ok(Some(ecash_key))
}

/// Load every archived ecash keypair sitting alongside the live one.
///
/// Credentials outlive the epoch that issued them, so the keys put aside by
/// [`archive_ecash_keypair`] have to be readable again after a restart. The epoch each one
/// belongs to is taken from the file *contents*; the name only locates the candidates.
///
/// Individual failures are logged and skipped rather than propagated: a single unreadable
/// archive must not stop this api from serving the epoch it is currently signing for.
pub(crate) fn load_archived_ecash_keypairs<P: AsRef<Path>>(store_path: P) -> Vec<KeyPairWithEpoch> {
    let store_path = store_path.as_ref();

    let (dir, live_filename) = match key_path_parts(store_path) {
        Ok(parts) => parts,
        Err(err) => {
            warn!("{err} - no archived ecash keys will be loaded");
            return Vec::new();
        }
    };

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            // an api that has never derived a key has no directory yet, which is not a problem
            debug!(
                "could not read the ecash key directory {}: {err} - no archived keys will be loaded",
                dir.display()
            );
            return Vec::new();
        }
    };

    let mut loaded = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(named_epoch) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| archived_key_epoch(live_filename, name))
        else {
            continue;
        };

        match nym_pemstore::load_key::<KeyPairWithEpoch, _>(&path) {
            Ok(keys) => {
                if keys.issued_for_epoch != named_epoch {
                    // the contents win: that's what the signatures would be produced with
                    warn!(
                        "the archived ecash key at {} was issued for epoch {} rather than the {named_epoch} its name claims",
                        path.display(),
                        keys.issued_for_epoch
                    );
                }
                debug!(
                    "loaded archived ecash keys for epoch {} from {}",
                    keys.issued_for_epoch,
                    path.display()
                );
                loaded.push(keys)
            }
            Err(err) => warn!(
                "failed to load the archived ecash key at {}: {err}. credentials from that epoch may not be servable",
                path.display()
            ),
        }
    }

    loaded
}

// the keys can be considered valid if they were generated for the current dkg epoch
// and we're either in the "in progress" or "key finalization" states of the DKG
pub(crate) async fn can_validate_ecash_keys(
    nyxd_client: &nyxd::Client,
    issued_for: EpochId,
) -> anyhow::Result<bool> {
    // validate the keys if they were generated for the current dkg epoch
    // and we're either in the "in progress" or "key finalization" states of the DKG
    let current_dkg_epoch = nyxd_client.get_current_epoch().await?;
    if issued_for != current_dkg_epoch.epoch_id {
        warn!("managed to load ecash keys, but they were generated for epoch {issued_for}. The current epoch is {}. the keys won't be used for credential issuance", current_dkg_epoch.epoch_id);
        Ok(false)
    } else if !matches!(
        current_dkg_epoch.state,
        EpochState::InProgress | EpochState::VerificationKeyFinalization { .. }
    ) {
        warn!("managed to load ecash keys, but the current DKG epoch is at {}. the keys won't (yet) be used for credential issuance", current_dkg_epoch.state);
        Ok(false)
    } else {
        Ok(true)
    }
}

pub(crate) fn persist_ecash_keypair<P: AsRef<Path>>(
    keys: &KeyPairWithEpoch,
    store_path: P,
) -> anyhow::Result<()> {
    nym_pemstore::store_key(keys, store_path).context("ecash key store failure")
}

pub(crate) fn archive_ecash_keypair<P: AsRef<Path>>(
    store_path: P,
    epoch_id: EpochId,
) -> anyhow::Result<()> {
    let store_path = store_path.as_ref();
    if !store_path.exists() {
        bail!("ecash key does not exist at {}", store_path.display())
    }

    let (dir, filename) = key_path_parts(store_path)?;
    std::fs::rename(
        store_path,
        dir.join(archived_key_filename(filename, epoch_id)),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_compact_ecash::ttp_keygen;
    use tempfile::tempdir;

    fn dummy_keys(epoch_id: EpochId) -> KeyPairWithEpoch {
        KeyPairWithEpoch::new(ttp_keygen(1, 1).unwrap().pop().unwrap(), epoch_id)
    }

    #[test]
    fn archived_key_names_round_trip() {
        let name = archived_key_filename("ecash.pem", 42);
        assert_eq!(name, "epoch-42-ecash.pem.archived");
        assert_eq!(archived_key_epoch("ecash.pem", &name), Some(42));

        // anything that isn't an archive of *this* key is not ours to load
        assert_eq!(archived_key_epoch("ecash.pem", "ecash.pem"), None);
        assert_eq!(
            archived_key_epoch("ecash.pem", "epoch-42-other.pem.archived"),
            None
        );
        assert_eq!(
            archived_key_epoch("ecash.pem", "epoch-2a-ecash.pem.archived"),
            None
        );
        assert_eq!(
            archived_key_epoch("ecash.pem", "epoch--ecash.pem.archived"),
            None
        );
    }

    #[test]
    fn archived_keys_are_loaded_back_by_epoch() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let key_path = dir.path().join("ecash.pem");

        for epoch_id in [3, 7] {
            persist_ecash_keypair(&dummy_keys(epoch_id), &key_path)?;
            archive_ecash_keypair(&key_path, epoch_id)?;
        }

        // the live key is not an archive, and neither is unrelated clutter
        persist_ecash_keypair(&dummy_keys(8), &key_path)?;
        std::fs::write(dir.path().join("persistent_state.json"), "{}")?;

        let mut epochs = load_archived_ecash_keypairs(&key_path)
            .into_iter()
            .map(|keys| keys.issued_for_epoch)
            .collect::<Vec<_>>();
        epochs.sort();
        assert_eq!(epochs, vec![3, 7]);

        Ok(())
    }

    #[test]
    fn an_unreadable_archive_does_not_hide_the_others() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let key_path = dir.path().join("ecash.pem");

        persist_ecash_keypair(&dummy_keys(1), &key_path)?;
        archive_ecash_keypair(&key_path, 1)?;
        std::fs::write(
            dir.path().join(archived_key_filename("ecash.pem", 2)),
            "not a key",
        )?;

        let loaded = load_archived_ecash_keypairs(&key_path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].issued_for_epoch, 1);

        Ok(())
    }

    #[test]
    fn a_missing_key_directory_yields_no_archives() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("never-created").join("ecash.pem");

        assert!(load_archived_ecash_keypairs(&key_path).is_empty());
    }
}
