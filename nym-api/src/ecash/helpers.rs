// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use nym_api_requests::ecash::BlindSignRequestBody;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use nym_compact_ecash::scheme::keygen::SecretKeyAuth;
use nym_compact_ecash::{BlindedSignature, EncodedDate, EncodedTicketType};
use nym_compact_ecash::{PublicKeyUser, WithdrawalRequest};
use nym_ecash_time::EcashTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::ops::Deref;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Serialize, Deserialize)]
pub(crate) struct IssuedExpirationDateSignatures {
    pub(crate) epoch_id: EpochId,
    pub(crate) signatures: Vec<AnnotatedExpirationDateSignature>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct IssuedCoinIndicesSignatures {
    pub(crate) epoch_id: EpochId,
    pub(crate) signatures: Vec<AnnotatedCoinIndexSignature>,
}

pub(crate) trait CredentialRequest {
    fn withdrawal_request(&self) -> &WithdrawalRequest;
    fn expiration_date_timestamp(&self) -> EncodedDate;
    fn ticketbook_type(&self) -> EncodedTicketType;
    fn ecash_pubkey(&self) -> PublicKeyUser;
}

impl CredentialRequest for BlindSignRequestBody {
    fn withdrawal_request(&self) -> &WithdrawalRequest {
        &self.inner_sign_request
    }

    fn expiration_date_timestamp(&self) -> EncodedDate {
        self.expiration_date.ecash_unix_timestamp()
    }

    fn ticketbook_type(&self) -> EncodedTicketType {
        self.ticketbook_type.encode()
    }

    fn ecash_pubkey(&self) -> PublicKeyUser {
        self.ecash_pubkey.clone()
    }
}

pub(crate) fn blind_sign<C: CredentialRequest>(
    request: &C,
    signing_key: &SecretKeyAuth,
) -> Result<BlindedSignature, EcashError> {
    Ok(nym_compact_ecash::scheme::withdrawal::issue(
        signing_key,
        request.ecash_pubkey().clone(),
        request.withdrawal_request(),
        request.expiration_date_timestamp(),
        request.ticketbook_type(),
    )?)
}

// a map of items that never change for given key
pub(crate) struct CachedImmutableItems<K, V> {
    // I wonder if there's a more efficient structure with OnceLock or OnceCell or something
    inner: RwLock<HashMap<K, V>>,
}

// an item that stays constant throughout given epoch
pub(crate) type CachedImmutableEpochItem<T> = CachedImmutableItems<EpochId, T>;

impl<K, V> Default for CachedImmutableItems<K, V> {
    fn default() -> Self {
        CachedImmutableItems {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl<K, V> Deref for CachedImmutableItems<K, V> {
    type Target = RwLock<HashMap<K, V>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K, V> CachedImmutableItems<K, V>
where
    K: Eq + Hash,
{
    pub(crate) async fn get_or_init<F, U>(
        &self,
        key: K,
        f: F,
    ) -> Result<RwLockReadGuard<V>, EcashError>
    where
        F: FnOnce() -> U,
        U: Future<Output = Result<V, EcashError>>,
        K: Clone,
    {
        // 1. see if we already have the item cached
        let guard = self.inner.read().await;
        if let Ok(item) = RwLockReadGuard::try_map(guard, |map| map.get(&key)) {
            return Ok(item);
        }

        // 2. attempt to retrieve (and cache) it
        let mut write_guard = self.inner.write().await;

        // see if another task has already set the item whilst we were waiting for the lock
        if write_guard.get(&key).is_some() {
            let read_guard = write_guard.downgrade();

            // SAFETY: we just checked the entry exists and we never dropped the guard
            #[allow(clippy::unwrap_used)]
            return Ok(RwLockReadGuard::map(read_guard, |map| {
                map.get(&key).unwrap()
            }));
        }

        let init = f().await?;
        write_guard.insert(key.clone(), init);

        let guard = write_guard.downgrade();

        // SAFETY:
        // we just inserted the entry into the map while NEVER dropping the lock (only downgraded it)
        // so it MUST exist and thus the unwrap is fine
        #[allow(clippy::unwrap_used)]
        Ok(RwLockReadGuard::map(guard, |map| map.get(&key).unwrap()))
    }

    pub(crate) async fn remove(&self, key: K) {
        self.inner.write().await.remove(&key);
    }
}
