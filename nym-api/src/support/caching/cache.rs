// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Debug, Error)]
#[error("the cache item has not been initialised")]
pub struct UninitialisedCache;

pub struct SharedCache<T>(Arc<RwLock<CachedItem<T>>>);

impl<T> Clone for SharedCache<T> {
    fn clone(&self) -> Self {
        SharedCache(Arc::clone(&self.0))
    }
}

impl<T> Default for SharedCache<T> {
    fn default() -> Self {
        SharedCache(Arc::new(RwLock::new(CachedItem { inner: None })))
    }
}

impl<T> SharedCache<T> {
    pub(crate) fn new() -> Self {
        SharedCache::default()
    }

    pub(crate) async fn update(&self, value: T) {
        let mut guard = self.0.write().await;
        if let Some(ref mut existing) = guard.inner {
            existing.unchecked_update(value)
        } else {
            guard.inner = Some(Cache::new(value))
        }
    }

    pub(crate) async fn get(&self) -> Result<RwLockReadGuard<'_, Cache<T>>, UninitialisedCache> {
        let guard = self.0.read().await;
        RwLockReadGuard::try_map(guard, |a| a.inner.as_ref()).map_err(|_| UninitialisedCache)
    }

    // ignores expiration data
    #[allow(dead_code)]
    pub(crate) async fn unchecked_get_inner(
        &self,
    ) -> Result<RwLockReadGuard<'_, T>, UninitialisedCache> {
        Ok(RwLockReadGuard::map(self.get().await?, |a| &a.value))
    }
}

impl<T> From<Cache<T>> for SharedCache<T> {
    fn from(value: Cache<T>) -> Self {
        SharedCache(Arc::new(RwLock::new(value.into())))
    }
}

#[derive(Default)]
pub(crate) struct CachedItem<T> {
    inner: Option<Cache<T>>,
}

impl<T> CachedItem<T> {
    #[allow(dead_code)]
    fn get(&self) -> Result<&Cache<T>, UninitialisedCache> {
        self.inner.as_ref().ok_or(UninitialisedCache)
    }
}

impl<T> From<Cache<T>> for CachedItem<T> {
    fn from(value: Cache<T>) -> Self {
        CachedItem { inner: Some(value) }
    }
}

// don't use this directly!
// opt for SharedCache<T> instead
pub struct Cache<T> {
    value: T,
    as_at: OffsetDateTime,
}

impl<T> Cache<T> {
    // ugh. I hate to expose it, but it'd have broken pre-existing code
    pub(crate) fn new(value: T) -> Self {
        Cache {
            value,
            as_at: OffsetDateTime::now_utc(),
        }
    }

    // I know, it's dead code for now, but I feel it could be useful code in the future
    #[allow(dead_code)]
    pub(crate) fn map<F, U>(this: Self, f: F) -> Cache<U>
    where
        F: FnOnce(T) -> U,
    {
        Cache {
            value: f(this.value),
            as_at: this.as_at,
        }
    }

    // ugh. I hate to expose it, but it'd have broken pre-existing code
    pub(crate) fn clone_cache(&self) -> Self
    where
        T: Clone,
    {
        Cache {
            value: self.value.clone(),
            as_at: self.as_at,
        }
    }

    // ugh. I hate to expose it, but it'd have broken pre-existing code
    pub(crate) fn unchecked_update(&mut self, value: T) {
        self.value = value;
        self.as_at = OffsetDateTime::now_utc()
    }

    #[allow(dead_code)]
    pub fn has_expired(&self, ttl: Duration, now: Option<OffsetDateTime>) -> bool {
        let now = now.unwrap_or(OffsetDateTime::now_utc());
        let diff = now - self.as_at;

        diff > ttl
    }

    pub fn timestamp(&self) -> OffsetDateTime {
        self.as_at
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> Deref for Cache<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Default for Cache<T>
where
    T: Default,
{
    fn default() -> Self {
        Cache {
            value: T::default(),
            as_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}
