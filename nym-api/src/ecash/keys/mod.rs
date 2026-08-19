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

    /// The keys derived for `epoch_id`, whether that is the epoch we're actively signing for
    /// or one that has since rotated.
    pub async fn keys_for_epoch(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, KeyPairWithEpoch>, EcashError> {
        let current = self.current_key_epoch().await;

        // the epoch we're actively signing for goes through the usual validity gate
        if current == Some(epoch_id) {
            return self.keys().await;
        }

        // any other epoch comes out of the archive, and is deliberately not subject to that
        // gate: it tracks whether the *current* keys may be used for issuance, and the chain
        // is the authority on whether an archived share was ever verified
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

    pub async fn keys(&self) -> Result<RwLockReadGuard<'_, KeyPairWithEpoch>, EcashError> {
        let keypair_guard = self.get().await.ok_or(EcashError::KeyPairNotDerivedYet)?;
        RwLockReadGuard::try_map(keypair_guard, |keypair| keypair.as_ref())
            .map_err(|_| EcashError::KeyPairNotDerivedYet)
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

    #[tokio::test]
    async fn the_epoch_we_sign_for_is_still_subject_to_the_validity_gate() {
        let keys = KeyPair::new();
        keys.set(dummy_keys(5)).await;

        // derived but not yet finalised on chain
        assert!(matches!(
            keys.keys_for_epoch(5).await,
            Err(EcashError::KeyPairNotDerivedYet)
        ));

        keys.validate();
        assert_eq!(keys.keys_for_epoch(5).await.unwrap().issued_for_epoch, 5);
    }

    /// The gate above tracks whether the keys we sign *with* may be used for issuance. An
    /// archived epoch is not covered by it: the chain already settled which shares were
    /// verified for that epoch, and its credentials need their material regardless of what
    /// the current ceremony is doing.
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
