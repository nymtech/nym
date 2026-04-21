// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumErrorResponse;
use crate::support::caching::refresher::RefreshRequester;
use crate::support::nyxd::Client;
use nym_validator_client::nyxd::error::NyxdError;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockWriteGuard};

/// Handle for on-demand cache refreshes driven by external triggers (e.g. an HTTP endpoint).
///
/// Wraps a [`RefreshRequester`] alongside the timestamp of the most recent refresh request so
/// callers can rate-limit or expose "last refreshed" information without reaching into the
/// underlying cache.
#[derive(Clone)]
pub(crate) struct Refreshing {
    handle: RefreshRequester,
    /// Unix timestamp of the last refresh request; stored atomically so multiple request handlers
    /// can update it concurrently without taking a lock.
    last_requested: Arc<AtomicI64>,
}

impl Refreshing {
    pub(crate) fn new(handle: RefreshRequester) -> Self {
        Refreshing {
            handle,
            last_requested: Arc::new(Default::default()),
        }
    }

    pub(crate) fn last_requested(&self) -> OffsetDateTime {
        // SAFETY: this value is always populated with valid timestamps
        #[allow(clippy::unwrap_used)]
        OffsetDateTime::from_unix_timestamp(self.last_requested.load(Ordering::SeqCst)).unwrap()
    }

    fn update_last_requested(&self, now: OffsetDateTime) {
        self.last_requested
            .store(now.unix_timestamp(), Ordering::SeqCst);
    }

    pub(crate) fn request_refresh(&self, now: OffsetDateTime) {
        self.update_last_requested(now);
        self.handle.request_cache_refresh();
    }
}

/// Shared, TTL-gated cache for values that are (re)hydrated from the nyxd chain on demand.
///
/// The cache collapses the common "check cache, otherwise refresh" pattern used across the various
/// chain-backed state caches (chain status, contract details, ...) into a single generic type.
/// Callers plug in a type-specific `refresh_fn` that knows how to fetch `T` from the chain; this
/// type handles the locking, TTL check, and single-flight behavior.
///
/// Concurrency model:
/// - Reads happen under a read lock; if the cached value is present and within TTL it is returned
///   immediately.
/// - If the cached value is missing or stale, a single writer takes the write lock, re-checks the
///   TTL (so a refresh that completed while we were waiting isn't redundantly repeated) and then
///   invokes `refresh_fn`. Other concurrent callers will block on the write lock and observe the
///   freshly populated value instead of each running their own query against the chain.
#[derive(Clone)]
pub(crate) struct ChainSharedCacheWithTtl<T> {
    cache_ttl: Duration,
    inner: Arc<RwLock<Option<ChainSharedCacheWithTtlInner<T>>>>,
}

impl<T> ChainSharedCacheWithTtl<T>
where
    T: Clone,
{
    pub(crate) fn new(cache_ttl: Duration) -> Self {
        ChainSharedCacheWithTtl {
            cache_ttl,
            inner: Arc::new(RwLock::new(None)),
        }
    }

    /// Return the cached value if it is still fresh, otherwise refresh it via `refresh_fn`.
    ///
    /// Takes the read-only fast path when the cache is warm and only escalates to a write-locked
    /// refresh when the value is missing or expired.
    pub(crate) async fn get_or_refresh<F>(
        &self,
        client: &Client,
        refresh_fn: F,
    ) -> Result<T, AxumErrorResponse>
    where
        F: AsyncFn(&Client) -> Result<T, NyxdError>,
    {
        if let Some(cached) = self.check_cache().await {
            return Ok(cached);
        }

        self.refresh(client, refresh_fn).await
    }

    /// Return the cached value if present and within TTL without attempting a refresh.
    async fn check_cache(&self) -> Option<T>
    where
        T: Clone,
    {
        let guard = self.inner.read().await;
        let inner = guard.as_ref()?;
        if inner.is_valid(self.cache_ttl) {
            return Some(inner.value.clone());
        }
        None
    }

    /// Forcibly re-query the chain via `refresh_fn` and replace the cached value.
    ///
    /// The double-checked TTL guard after acquiring the write lock prevents the common
    /// thundering-herd case where many concurrent callers all observe a stale cache at once - only
    /// the first one to acquire the write lock will actually hit the chain.
    async fn refresh<F>(&self, client: &Client, refresh_fn: F) -> Result<T, AxumErrorResponse>
    where
        F: AsyncFn(&Client) -> Result<T, NyxdError>,
        T: Clone,
    {
        // 1. attempt to get write lock permit
        let mut guard = self.inner.write().await;

        // 2. check if another task hasn't already updated the cache whilst we were waiting for the permit
        if let Some(cached) = guard.as_ref() {
            if cached.is_valid(self.cache_ttl) {
                return Ok(cached.clone_value());
            }
        }

        let refresh_res = refresh_fn(client).await?;

        *guard = Self::new_inner(refresh_res.clone());
        Ok(refresh_res)
    }

    fn new_inner(value: T) -> Option<ChainSharedCacheWithTtlInner<T>> {
        Some(ChainSharedCacheWithTtlInner::new(value))
    }
}

/// Cached value alongside the timestamp at which it was fetched, used to evaluate the TTL.
struct ChainSharedCacheWithTtlInner<T> {
    last_refreshed_at: OffsetDateTime,
    value: T,
}

impl<T> ChainSharedCacheWithTtlInner<T> {
    fn new(value: T) -> Self {
        ChainSharedCacheWithTtlInner {
            last_refreshed_at: OffsetDateTime::now_utc(),
            value,
        }
    }

    fn is_valid(&self, ttl: Duration) -> bool {
        self.last_refreshed_at + ttl > OffsetDateTime::now_utc()
    }

    fn clone_value(&self) -> T
    where
        T: Clone,
    {
        self.value.clone()
    }
}
