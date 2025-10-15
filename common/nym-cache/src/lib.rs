// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::ops::Deref;
use tokio::sync::{RwLock, RwLockReadGuard};

/// A map of items that never change for given key
pub struct CachedImmutableItems<K, V> {
    // I wonder if there's a more efficient structure with OnceLock or OnceCell or something
    inner: RwLock<HashMap<K, V>>,
}

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
    pub async fn get_or_init<F, U, E>(
        &self,
        key: K,
        initialiser: F,
    ) -> Result<RwLockReadGuard<'_, V>, E>
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

        let init = initialiser().await?;
        write_guard.insert(key.clone(), init);

        let guard = write_guard.downgrade();

        // SAFETY:
        // we just inserted the entry into the map while NEVER dropping the lock (only downgraded it)
        // so it MUST exist and thus the unwrap is fine
        #[allow(clippy::unwrap_used)]
        Ok(RwLockReadGuard::map(guard, |map| map.get(&key).unwrap()))
    }
}
