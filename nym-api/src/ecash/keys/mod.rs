// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::{SecretKeyAuth, VerificationKeyAuth};
use nym_dkg::Scalar;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

mod persistence;

#[derive(Clone, Debug)]
pub struct KeyPair {
    keys: Arc<RwLock<Option<KeyPairWithEpoch>>>,

    /// Keys derived for epochs that have since rotated. Credentials outlive the epoch that
    /// issued them, so their auxiliary signatures have to remain producible afterwards.
    archived: Arc<RwLock<HashMap<EpochId, KeyPairWithEpoch>>>,

    valid: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct KeyPairWithEpoch {
    pub(crate) keys: nym_compact_ecash::KeyPairAuth,
    pub(crate) issued_for_epoch: EpochId,
}

impl KeyPairWithEpoch {
    pub(crate) fn new(keys: nym_compact_ecash::KeyPairAuth, issued_for_epoch: EpochId) -> Self {
        KeyPairWithEpoch {
            keys,
            issued_for_epoch,
        }
    }

    // extract underlying secrets from the coconut's secret key.
    // the caller of this function must exercise extreme care to not misuse the data and ensuring it gets zeroized
    // `KeyPair` and `SecretKey` implement ZeroizeOnDrop; `Scalar` does not (it implements `Copy` -> important to keep in mind)
    //
    // this borrows rather than consumes because the keypair outlives the resharing it feeds:
    // it gets archived for the epoch it was issued for, whose credentials still need it
    pub(crate) fn signing_key(&self) -> &SecretKeyAuth {
        self.keys.secret_key()
    }

    pub(crate) fn hazmat_secrets(&self) -> Vec<Scalar> {
        let (x, mut secrets) = self.keys.secret_key().hazmat_to_raw();

        secrets.insert(0, x);
        secrets
    }
}

impl KeyPair {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(None)),
            archived: Arc::new(RwLock::new(HashMap::new())),
            valid: Arc::new(Default::default()),
        }
    }

    pub async fn take(&self) -> Option<KeyPairWithEpoch> {
        self.keys.write().await.take()
    }

    /// Retain a keypair belonging to an epoch that is no longer current, so that credentials
    /// issued under it can still be served their auxiliary signatures.
    pub async fn archive(&self, keypair: KeyPairWithEpoch) {
        let epoch_id = keypair.issued_for_epoch;
        self.archived.write().await.insert(epoch_id, keypair);
    }

    /// The epoch our currently held keys were derived for, regardless of whether they may
    /// yet be used for issuance.
    async fn current_key_epoch(&self) -> Option<EpochId> {
        self.keys
            .read()
            .await
            .as_ref()
            .map(|keys| keys.issued_for_epoch)
    }

    /// The keys derived for `epoch_id`, wherever we happen to be keeping them - the live slot
    /// if it is still the epoch we sign for, otherwise the archive.
    ///
    /// This is a lookup and nothing more: it deliberately does **not** consult [`Self::valid`],
    /// which answers a different question ("may we issue credentials right now"). A rotation
    /// clears that flag the moment the next ceremony starts, while the keys it clears it for
    /// stay in the live slot until dealing exchange moves them to the archive - so a gate here
    /// would refuse a settled epoch's auxiliary signatures for exactly as long as that window
    /// lasts, and serve them either side of it.
    ///
    /// Callers are responsible for establishing that they may use these keys at all;
    /// `EcashState::ensure_ceremony_concluded` is how the signature paths do it.
    pub async fn keys_for_epoch(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, KeyPairWithEpoch>, EcashError> {
        let current = self.current_key_epoch().await;

        if current == Some(epoch_id) {
            return RwLockReadGuard::try_map(self.read_keys().await, |keys| keys.as_ref())
                .map_err(|_| EcashError::KeyPairNotDerivedYet);
        }

        RwLockReadGuard::try_map(self.archived.read().await, |archived| {
            archived.get(&epoch_id)
        })
        .map_err(|_| match current {
            Some(available) => EcashError::InvalidSigningKeyEpoch {
                requested: epoch_id,
                available,
            },
            None => EcashError::KeyPairNotDerivedYet,
        })
    }

    pub async fn get(&self) -> Option<RwLockReadGuard<'_, Option<KeyPairWithEpoch>>> {
        if self.is_valid() {
            Some(self.read_keys().await)
        } else {
            None
        }
    }

    pub async fn signing_key(&self) -> Result<RwLockReadGuard<'_, SecretKeyAuth>, EcashError> {
        let keypair_guard = self.get().await.ok_or(EcashError::KeyPairNotDerivedYet)?;

        RwLockReadGuard::try_map(keypair_guard, |keypair| {
            keypair.as_ref().map(|k| k.keys.secret_key())
        })
        .map_err(|_| EcashError::KeyPairNotDerivedYet)
    }

    pub async fn verification_key(&self) -> Option<RwLockReadGuard<'_, VerificationKeyAuth>> {
        RwLockReadGuard::try_map(self.get().await?, |maybe_keys| {
            maybe_keys.as_ref().map(|k| k.keys.verification_key_ref())
        })
        .ok()
    }

    pub async fn read_keys(&self) -> RwLockReadGuard<'_, Option<KeyPairWithEpoch>> {
        self.keys.read().await
    }

    pub async fn set(&self, keypair: KeyPairWithEpoch) {
        let mut w_lock = self.keys.write().await;
        *w_lock = Some(keypair);
    }

    pub fn is_valid(&self) -> bool {
        self.valid.load(Ordering::SeqCst)
    }

    pub fn validate(&self) {
        self.valid.store(true, Ordering::SeqCst);
    }

    pub fn invalidate(&self) {
        self.valid.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_compact_ecash::ttp_keygen;

    fn dummy_keys(epoch_id: EpochId) -> KeyPairWithEpoch {
        KeyPairWithEpoch::new(ttp_keygen(1, 1).unwrap().pop().unwrap(), epoch_id)
    }

    /// The lookup is only about where the keys are, never about whether we may issue with them.
    ///
    /// That distinction is load bearing: a rotation clears `valid` when the next ceremony
    /// starts, but the keys it clears it for stay in the live slot until dealing exchange
    /// archives them. A gate here would refuse a settled epoch's auxiliary signatures for
    /// precisely that window and serve them either side of it.
    #[tokio::test]
    async fn the_lookup_does_not_care_whether_the_keys_may_be_used_for_issuance() {
        let keys = KeyPair::new();
        keys.set(dummy_keys(5)).await;

        // never validated, e.g. derived but not yet finalised on chain
        assert_eq!(keys.keys_for_epoch(5).await.unwrap().issued_for_epoch, 5);

        keys.validate();
        assert_eq!(keys.keys_for_epoch(5).await.unwrap().issued_for_epoch, 5);

        // and once a later ceremony has cleared the flag again
        keys.invalidate();
        assert_eq!(keys.keys_for_epoch(5).await.unwrap().issued_for_epoch, 5);
    }

    #[tokio::test]
    async fn an_archived_epoch_is_served_whatever_the_current_keys_are_doing() {
        let keys = KeyPair::new();
        keys.set(dummy_keys(5)).await;
        keys.archive(dummy_keys(4)).await;

        assert_eq!(keys.keys_for_epoch(4).await.unwrap().issued_for_epoch, 4);

        // ... including in the middle of the next ceremony, when the live keys are unusable
        keys.invalidate();
        assert_eq!(keys.keys_for_epoch(4).await.unwrap().issued_for_epoch, 4);
    }

    /// Issuance keeps its own gate, and it is the only thing the flag governs.
    #[tokio::test]
    async fn issuance_is_still_refused_while_the_keys_are_not_usable() {
        let keys = KeyPair::new();
        keys.set(dummy_keys(5)).await;

        assert!(keys.signing_key().await.is_err());
        assert!(keys.verification_key().await.is_none());

        keys.validate();
        assert!(keys.signing_key().await.is_ok());

        keys.invalidate();
        assert!(keys.signing_key().await.is_err());
    }

    #[tokio::test]
    async fn an_epoch_we_never_held_keys_for_is_refused() {
        let keys = KeyPair::new();

        // nothing at all, so there is no epoch to report as the one we do have
        assert!(matches!(
            keys.keys_for_epoch(4).await,
            Err(EcashError::KeyPairNotDerivedYet)
        ));

        keys.set(dummy_keys(5)).await;
        keys.validate();
        keys.archive(dummy_keys(4)).await;

        assert!(matches!(
            keys.keys_for_epoch(3).await,
            Err(EcashError::InvalidSigningKeyEpoch {
                requested: 3,
                available: 5
            })
        ));
    }
}
