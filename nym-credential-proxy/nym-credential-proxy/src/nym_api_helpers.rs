// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: this was just copied from nym-api;
// it should have been therefore extracted to a common crate instead and imported as dependency

use crate::error::VpnApiError;
use futures::{stream, StreamExt};
use nym_credentials::ecash::utils::{cred_exp_date, ecash_today};
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::Epoch;
use nym_validator_client::EcashApiClient;
use std::cmp::min;
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::ops::Deref;
use time::{Date, OffsetDateTime};
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};
use tracing::warn;

pub(crate) struct CachedEpoch {
    valid_until: OffsetDateTime,
    pub(crate) current_epoch: Epoch,
}

impl Default for CachedEpoch {
    fn default() -> Self {
        CachedEpoch {
            valid_until: OffsetDateTime::UNIX_EPOCH,
            current_epoch: Epoch::default(),
        }
    }
}

impl CachedEpoch {
    pub(crate) fn is_valid(&self) -> bool {
        self.valid_until > OffsetDateTime::now_utc()
    }

    pub(crate) fn update(&mut self, epoch: Epoch) {
        let now = OffsetDateTime::now_utc();

        let validity_duration = if let Some(epoch_finish) = epoch.deadline {
            #[allow(clippy::unwrap_used)]
            let state_end =
                OffsetDateTime::from_unix_timestamp(epoch_finish.seconds() as i64).unwrap();
            let until_epoch_state_end = state_end - now;
            // make it valid until the next epoch transition or next 5min, whichever is smaller
            min(until_epoch_state_end, 5 * time::Duration::MINUTE)
        } else {
            5 * time::Duration::MINUTE
        };

        self.valid_until = now + validity_duration;
        self.current_epoch = epoch;
    }
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
    pub(crate) async fn get_or_init<F, U, E>(&self, key: K, f: F) -> Result<RwLockReadGuard<V>, E>
    where
        F: FnOnce() -> U,
        U: Future<Output = Result<V, E>>,
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
}

pub(crate) fn ensure_sane_expiration_date(expiration_date: Date) -> Result<(), VpnApiError> {
    let today = ecash_today();

    if expiration_date < today.date() {
        // what's the point of signatures with expiration in the past?
        return Err(VpnApiError::ExpirationDateTooEarly);
    }

    // SAFETY: we're nowhere near MAX date
    #[allow(clippy::unwrap_used)]
    if expiration_date > cred_exp_date().date().next_day().unwrap() {
        // don't allow issuing signatures too far in advance (1 day beyond current value is fine)
        return Err(VpnApiError::ExpirationDateTooLate);
    }

    Ok(())
}

pub(crate) async fn query_all_threshold_apis<F, T, U>(
    all_apis: Vec<EcashApiClient>,
    threshold: u64,
    f: F,
) -> Result<Vec<T>, VpnApiError>
where
    F: Fn(EcashApiClient) -> U,
    U: Future<Output = Result<T, VpnApiError>>,
{
    let shares = Mutex::new(Vec::with_capacity(all_apis.len()));

    stream::iter(all_apis)
        .for_each_concurrent(8, |api| async {
            // can't be bothered to restructure the code to appease the borrow checker properly,
            // so just assign this to a variable
            let disp = api.to_string();
            match f(api).await {
                Ok(partial_share) => shares.lock().await.push(partial_share),
                Err(err) => {
                    warn!("failed to obtain partial threshold data from API: {disp}: {err}")
                }
            }
        })
        .await;

    let shares = shares.into_inner();

    if shares.len() < threshold as usize {
        return Err(VpnApiError::InsufficientNumberOfSigners {
            threshold,
            available: shares.len(),
        });
    }

    Ok(shares)
}
