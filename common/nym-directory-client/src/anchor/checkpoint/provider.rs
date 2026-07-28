// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::checkpoint::store::CheckpointStore;
use crate::anchor::checkpoint::{Checkpoint, SignedCheckpoint};
use crate::error::DirectoryClientError;
use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519;
use time::OffsetDateTime;
use tracing::warn;

/// One source of a seed checkpoint. Yields a checkpoint that this source vouches for (a signed
/// source verifies the root signature; the stored source trusts local disk), or `None` if it
/// has nothing usable. Staleness is NOT checked here - that is the loader's job.
#[async_trait]
pub trait CheckpointProvider: Send + Sync {
    async fn candidate(&self) -> Option<Checkpoint>;
}

/// Highest-priority source: the locally persisted, previously light-client-verified head. It
/// carries no root signature - it is trusted transitively (it was verified forward from a
/// root-anchored seed) at local-filesystem-integrity level.
pub struct StoredCheckpointProvider<S> {
    store: S,
}

impl<S: CheckpointStore> StoredCheckpointProvider<S> {
    pub fn new(store: S) -> Self {
        StoredCheckpointProvider { store }
    }
}

#[async_trait]
impl<S: CheckpointStore> CheckpointProvider for StoredCheckpointProvider<S> {
    async fn candidate(&self) -> Option<Checkpoint> {
        self.store.load()
    }
}

/// The compiled-in seed: a root-signed datum from a `nym-network-defaults` constant. An empty
/// datum means "no compiled checkpoint" (the loader falls through to the next source).
pub struct HardcodedCheckpointProvider {
    root: ed25519::PublicKey,
    datum_json: String,
}

impl HardcodedCheckpointProvider {
    pub fn new(root: ed25519::PublicKey, datum_json: impl Into<String>) -> Self {
        HardcodedCheckpointProvider {
            root,
            datum_json: datum_json.into(),
        }
    }
}

/// Parse a JSON-encoded [`SignedCheckpoint`] constant and return its checkpoint iff the root
/// signature verifies. An empty string is treated as "absent" (no log) - the hardcoded source's
/// "no compiled checkpoint" state.
fn parse_and_verify_datum(datum_json: &str, root: &ed25519::PublicKey) -> Option<Checkpoint> {
    if datum_json.trim().is_empty() {
        return None;
    }
    match serde_json::from_str::<SignedCheckpoint>(datum_json) {
        Ok(signed) => signed.verify_from_source(root, "hardcoded"),
        Err(err) => {
            warn!("ignoring malformed hardcoded checkpoint datum: {err}");
            None
        }
    }
}

#[async_trait]
impl CheckpointProvider for HardcodedCheckpointProvider {
    async fn candidate(&self) -> Option<Checkpoint> {
        parse_and_verify_datum(&self.datum_json, &self.root)
    }
}

/// Try `providers` in order and return the first candidate that is within the trusting period
/// at `now`. Returns [`DirectoryClientError::NoValidCheckpointSource`] if none qualifies.
pub async fn load_checkpoint(
    providers: &[&dyn CheckpointProvider],
    now: OffsetDateTime,
) -> Result<Checkpoint, DirectoryClientError> {
    for provider in providers {
        if let Some(candidate) = provider.candidate().await {
            let height = candidate.height;
            if candidate.is_stale(now) {
                warn!("skipping a stale checkpoint candidate at height {height}",);
                continue;
            }
            return Ok(candidate);
        }
    }
    Err(DirectoryClientError::NoValidCheckpointSource)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchor::checkpoint::fetcher::{MockFetcher, mock_https_provider};
    use crate::anchor::checkpoint::store::InMemoryCheckpointStore;
    use crate::test_support::{checkpoint, signed_checkpoint, signed_datum};
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use time::macros::datetime;

    // the fixture's block time is 2026-07-02T13:42:10Z; these bracket the 18-day trusting period
    const FRESH_TS: OffsetDateTime = datetime!(2026-07-05 00:00:00+00:00);
    const STALE_TS: OffsetDateTime = datetime!(2026-07-25 00:00:00+00:00);
    const MINTED_AT: OffsetDateTime = datetime!(2026-07-02 13:42:10+00:00);

    #[tokio::test]
    async fn stored_head_is_preferred_over_hardcoded() {
        let root = dummy_ed25519_keypair(1);
        // a distinguishable stored head (bumped height)
        let mut stored_cp = checkpoint();
        stored_cp.height = stored_cp.height.increment();
        let store = InMemoryCheckpointStore::default();
        store.save(&stored_cp);

        let stored = StoredCheckpointProvider::new(store);
        let hardcoded =
            HardcodedCheckpointProvider::new(*root.public_key(), signed_datum(&root, MINTED_AT));

        let got = load_checkpoint(&[&stored, &hardcoded], FRESH_TS)
            .await
            .unwrap();
        assert_eq!(got.height, stored_cp.height); // stored won
    }

    #[tokio::test]
    async fn falls_back_to_hardcoded_when_stored_absent() {
        let root = dummy_ed25519_keypair(1);
        let stored = StoredCheckpointProvider::new(InMemoryCheckpointStore::default());
        let hardcoded =
            HardcodedCheckpointProvider::new(*root.public_key(), signed_datum(&root, MINTED_AT));

        let got = load_checkpoint(&[&stored, &hardcoded], FRESH_TS)
            .await
            .unwrap();
        assert_eq!(got.height, checkpoint().height);
    }

    #[tokio::test]
    async fn empty_hardcoded_datum_is_absent() {
        let root = dummy_ed25519_keypair(1);
        let hardcoded = HardcodedCheckpointProvider::new(*root.public_key(), "");
        assert!(hardcoded.candidate().await.is_none());
    }

    #[tokio::test]
    async fn hardcoded_with_a_bad_root_signature_is_ignored() {
        let real_root = dummy_ed25519_keypair(1);
        let impostor = dummy_ed25519_keypair(2);
        // datum signed by the impostor, verified against the real root
        let hardcoded = HardcodedCheckpointProvider::new(
            *real_root.public_key(),
            signed_datum(&impostor, MINTED_AT),
        );
        assert!(hardcoded.candidate().await.is_none());

        let err = load_checkpoint(&[&hardcoded], FRESH_TS).await.unwrap_err();
        assert!(matches!(err, DirectoryClientError::NoValidCheckpointSource));
    }

    #[tokio::test]
    async fn stale_candidates_yield_no_source() {
        let root = dummy_ed25519_keypair(1);
        let hardcoded =
            HardcodedCheckpointProvider::new(*root.public_key(), signed_datum(&root, MINTED_AT));
        let err = load_checkpoint(&[&hardcoded], STALE_TS).await.unwrap_err();
        assert!(matches!(err, DirectoryClientError::NoValidCheckpointSource));
    }

    #[test]
    fn checkpoint_store_round_trips() {
        let store = InMemoryCheckpointStore::default();
        assert!(store.load().is_none());
        let cp = checkpoint();
        store.save(&cp);
        assert_eq!(store.load().unwrap().height, cp.height);
    }

    #[tokio::test]
    async fn aged_out_seeds_fall_back_to_https() {
        let root = dummy_ed25519_keypair(1);
        // stored empty + hardcoded empty -> loader must reach the HTTPS source
        let stored = StoredCheckpointProvider::new(InMemoryCheckpointStore::default());
        let hardcoded = HardcodedCheckpointProvider::new(*root.public_key(), "");
        let fetcher = MockFetcher(Some(signed_checkpoint(&root, MINTED_AT)));
        let https = mock_https_provider(fetcher, &root);

        let got = load_checkpoint(&[&stored, &hardcoded, &https], FRESH_TS)
            .await
            .unwrap();
        assert_eq!(got.height, checkpoint().height);
    }
}
