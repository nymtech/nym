// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::cache::SharedCache;
use crate::support::caching::CacheNotification;
use async_trait::async_trait;
use nym_task::ShutdownToken;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::futures::Notified;
use tokio::sync::{watch, Notify};
use tokio::time::interval;
use tracing::{debug, error, info, trace, warn};

pub(crate) type CacheUpdateWatcher = watch::Receiver<CacheNotification>;

/// How soon a failed refresh is retried while a cache has never been built, so that a transient
/// failure at startup costs seconds rather than a whole refresh interval.
pub(crate) const FIRST_BUILD_RETRY_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct RefreshRequester(Arc<Notify>);

impl RefreshRequester {
    pub(crate) fn request_cache_refresh(&self) {
        self.0.notify_waiters()
    }

    pub(crate) fn notified(&self) -> Notified<'_> {
        self.0.notified()
    }
}

impl Default for RefreshRequester {
    fn default() -> Self {
        RefreshRequester(Arc::new(Notify::new()))
    }
}

/// Explanation on generics:
/// the internal SharedCache<T> can be updated in two ways
/// by default CacheItemProvider will just provide a T and the internal values will be swapped
/// however, an alternative is to make it provide another value of type S with an explicit update closure
/// this way the cache will be updated with a custom method mutating the existing value
/// the reason for this is to allow partial updates of maps, where we might not want to retrieve
/// the entire value, and we might want to just insert a new entry
pub struct CacheRefresher<T, E, S = T> {
    name: String,

    /// Path to an on-disk location where the contents of the retrieved items should be written
    /// upon refresh
    on_disk_file: Option<PathBuf>,
    refreshing_interval: Duration,
    refresh_notification_sender: watch::Sender<CacheNotification>,

    // it's not really THAT complex... it's just a boxed function
    #[allow(clippy::type_complexity)]
    update_fn: Option<Box<dyn Fn(&mut T, S) + Send + Sync>>,

    // TODO: the Send + Sync bounds are only required for the `start` method. could we maybe make it less restrictive?
    provider: Box<dyn CacheItemProvider<Error = E, Item = S> + Send + Sync>,
    shared_cache: SharedCache<T>,
    refresh_requester: RefreshRequester,
}

#[async_trait]
pub(crate) trait CacheItemProvider {
    type Item;
    type Error: std::error::Error;

    async fn wait_until_ready(&self) {}

    async fn try_refresh(&mut self) -> Result<Option<Self::Item>, Self::Error>;
}

// Generics explanation:
// T: the actual type held in the cache
// E: Error type associated with refresh failure
// S: data type retrieved during update operation. it must be convertible into T
// (so that initial state could be established or when no `custom_fn` is set)
impl<T, E, S> CacheRefresher<T, E, S>
where
    E: std::error::Error,
    S: Into<T>,
    T: Serialize,
{
    pub(crate) fn new_boxed(
        item_provider: Box<dyn CacheItemProvider<Error = E, Item = S> + Send + Sync>,
        refreshing_interval: Duration,
    ) -> Self {
        let (refresh_notification_sender, _) = watch::channel(CacheNotification::Start);

        CacheRefresher {
            name: "GenericCacheRefresher".to_string(),
            on_disk_file: None,
            refreshing_interval,
            refresh_notification_sender,
            update_fn: None,
            provider: item_provider,
            shared_cache: SharedCache::new(),
            refresh_requester: Default::default(),
        }
    }
    pub(crate) fn new<P>(item_provider: P, refreshing_interval: Duration) -> Self
    where
        P: CacheItemProvider<Error = E, Item = S> + Send + Sync + 'static,
    {
        Self::new_boxed(Box::new(item_provider), refreshing_interval)
    }

    pub(crate) fn new_with_initial_value(
        item_provider: Box<dyn CacheItemProvider<Error = E, Item = S> + Send + Sync>,
        refreshing_interval: Duration,
        shared_cache: SharedCache<T>,
    ) -> Self {
        let (refresh_notification_sender, _) = watch::channel(CacheNotification::Start);

        CacheRefresher {
            name: "GenericCacheRefresher".to_string(),
            on_disk_file: None,
            refreshing_interval,
            refresh_notification_sender,
            update_fn: None,
            provider: item_provider,
            shared_cache,
            refresh_requester: Default::default(),
        }
    }

    /// Rather than performing default behaviour of overwriting all existing values in the cache,
    /// provide a custom update function that will define the update behaviour.
    #[must_use]
    pub(crate) fn with_update_fn(
        mut self,
        update_fn: impl Fn(&mut T, S) + Send + Sync + 'static,
    ) -> Self {
        self.update_fn = Some(Box::new(update_fn));
        self
    }

    #[must_use]
    pub(crate) fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    #[must_use]
    pub(crate) fn with_persistent_cache(mut self, storage_path: PathBuf) -> Self {
        self.on_disk_file = Some(storage_path);
        self
    }

    pub(crate) fn update_watcher(&self) -> CacheUpdateWatcher {
        self.refresh_notification_sender.subscribe()
    }

    pub(crate) fn refresh_requester(&self) -> RefreshRequester {
        self.refresh_requester.clone()
    }

    #[allow(dead_code)]
    pub(crate) fn get_shared_cache(&self) -> SharedCache<T> {
        self.shared_cache.clone()
    }

    async fn update_cache(&self, mut update: S, update_fn: impl Fn(&mut T, S)) {
        let mut failures = 0;

        loop {
            match self
                .shared_cache
                .try_update_value(update, &update_fn, &self.name)
                .await
            {
                Ok(_) => break,
                Err(returned) => {
                    failures += 1;
                    update = returned
                }
            };
            if failures % 10 == 0 {
                warn!(
                    "failed to obtain write permit for {} cache {failures} times in a row!",
                    self.name
                );
            }

            tokio::time::sleep(Duration::from_secs_f32(0.5)).await
        }
    }

    async fn overwrite_cache(&self, mut updated_items: T) {
        let mut failures = 0;

        loop {
            match self
                .shared_cache
                .try_overwrite_old_value(updated_items, &self.name)
                .await
            {
                Ok(_) => break,
                Err(returned) => {
                    failures += 1;
                    updated_items = returned
                }
            };
            if failures % 10 == 0 {
                warn!(
                    "failed to obtain write permit for {} cache {failures} times in a row!",
                    self.name
                );
            }

            tokio::time::sleep(Duration::from_secs_f32(0.5)).await
        }
    }

    async fn try_flush_to_disk(&self) {
        // if specified, attempt to flush data onto disk
        let Some(disk_cache) = self.on_disk_file.as_ref() else {
            return;
        };
        let Ok(new_cached) = self.shared_cache.get().await else {
            error!("the {} cache is still not initialised!", self.name);
            return;
        };
        // error reporting is handled by the serialise function itself
        let _ = new_cached.try_serialise_to_file(disk_cache);
    }

    async fn do_refresh_cache(&mut self) {
        let updated_items = match self.provider.try_refresh().await {
            Err(err) => {
                error!("{}: failed to refresh the cache: {err}", self.name);
                return;
            }
            Ok(Some(items)) => items,
            Ok(None) => {
                debug!("no updates for {} cache this iteration", self.name);
                return;
            }
        };

        if let Some(update_fn) = self.update_fn.as_ref() {
            self.update_cache(updated_items, update_fn).await;
        } else {
            self.overwrite_cache(updated_items.into()).await;
        }

        self.try_flush_to_disk().await;

        if !self.refresh_notification_sender.is_closed()
            && self
                .refresh_notification_sender
                .send(CacheNotification::Updated)
                .is_err()
        {
            warn!("failed to send cache update notification");
        }
    }

    pub async fn refresh(&mut self, shutdown_token: &ShutdownToken) {
        info!("{}: refreshing cache state", self.name);

        tokio::select! {
            biased;
            _ = shutdown_token.cancelled() => {
                trace!("{}: Received shutdown while refreshing cache", self.name)
            }
            _ = self.do_refresh_cache() => (),
        }
    }

    pub async fn run(&mut self, shutdown_token: ShutdownToken) {
        self.provider.wait_until_ready().await;
        self.try_flush_to_disk().await;

        let mut refresh_interval = interval(self.refreshing_interval);
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    trace!("{}: Received shutdown", self.name);
                    break
                }
                _ = refresh_interval.tick() => self.refresh(&shutdown_token).await,
                // note: `Notify` is not cancellation safe, HOWEVER, there's only one listener,
                // so it doesn't matter if we lose our queue position
                _ = self.refresh_requester.notified() => {
                    self.refresh(&shutdown_token).await;
                    // since we just performed the full request, we can reset our existing interval
                    refresh_interval.reset();
                }
            }
            if self.shared_cache.get().await.is_err() {
                refresh_interval.reset_after(FIRST_BUILD_RETRY_INTERVAL);
            }
        }
    }

    pub fn start(mut self, shutdown_token: ShutdownToken)
    where
        T: Send + Sync + 'static,
        E: Send + Sync + 'static,
        S: Send + Sync + 'static,
    {
        tokio::spawn(async move { self.run(shutdown_token).await });
    }

    pub fn start_with_delay(mut self, shutdown_token: ShutdownToken, delay: Duration)
    where
        T: Send + Sync + 'static,
        E: Send + Sync + 'static,
        S: Send + Sync + 'static,
    {
        tokio::spawn(async move {
            let sleep = tokio::time::sleep(delay);
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    trace!("{}: Received shutdown", self.name);
                    return
                }
                _ = sleep => {},
            }
            self.run(shutdown_token).await
        });
    }

    pub fn start_with_watcher(self, shutdown_token: ShutdownToken) -> CacheUpdateWatcher
    where
        T: Send + Sync + 'static,
        E: Send + Sync + 'static,
        S: Send + Sync + 'static,
    {
        let receiver = self.update_watcher();
        self.start(shutdown_token);
        receiver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use tokio::time::Instant;

    const REFRESH_INTERVAL: Duration = Duration::from_secs(600);

    #[derive(Debug, thiserror::Error)]
    #[error("the refresh failed")]
    struct RefreshFailed;

    /// Fails or succeeds as scripted, succeeding once the script runs out, recording when each
    /// refresh was attempted.
    struct ScriptedProvider {
        outcomes: VecDeque<bool>,
        attempts: Arc<Mutex<Vec<Instant>>>,
    }

    #[async_trait]
    impl CacheItemProvider for ScriptedProvider {
        type Item = u32;
        type Error = RefreshFailed;

        async fn try_refresh(&mut self) -> Result<Option<u32>, RefreshFailed> {
            self.attempts.lock().unwrap().push(Instant::now());
            if self.outcomes.pop_front().unwrap_or(true) {
                Ok(Some(1))
            } else {
                Err(RefreshFailed)
            }
        }
    }

    /// When, from its start over `cache`, a refresher whose refreshes go as `outcomes` attempts
    /// them during the first `elapsed`.
    async fn attempts(
        cache: SharedCache<u32>,
        outcomes: &[bool],
        elapsed: Duration,
    ) -> Vec<Duration> {
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let provider = ScriptedProvider {
            outcomes: outcomes.iter().copied().collect(),
            attempts: attempts.clone(),
        };
        let mut refresher = CacheRefresher::<u32, RefreshFailed>::new_with_initial_value(
            Box::new(provider),
            REFRESH_INTERVAL,
            cache,
        );

        let shutdown = ShutdownToken::new();
        let start = Instant::now();
        let run = tokio::spawn({
            let shutdown = shutdown.clone();
            async move { refresher.run(shutdown).await }
        });
        tokio::time::sleep(elapsed).await;
        shutdown.cancel();
        run.await.unwrap();

        let attempts = attempts.lock().unwrap();
        attempts.iter().map(|attempt| *attempt - start).collect()
    }

    fn secs(offsets: &[u64]) -> Vec<Duration> {
        offsets.iter().copied().map(Duration::from_secs).collect()
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_first_build_is_retried_after_30_seconds() {
        let attempts = attempts(
            SharedCache::new(),
            &[false, false, true],
            Duration::from_secs(700),
        )
        .await;

        assert_eq!(attempts, secs(&[0, 30, 60, 660]));
    }

    #[tokio::test(start_paused = true)]
    async fn after_the_first_build_a_failure_waits_for_the_normal_interval() {
        let attempts = attempts(
            SharedCache::new(),
            &[true, false],
            Duration::from_secs(1300),
        )
        .await;

        assert_eq!(attempts, secs(&[0, 600, 1200]));
    }

    /// As after loading a persisted value within its age limit.
    #[tokio::test(start_paused = true)]
    async fn a_cache_holding_a_value_retries_a_failure_at_the_normal_interval() {
        let cache = SharedCache::new_with_value(1);
        let attempts = attempts(cache, &[false, false], Duration::from_secs(700)).await;

        assert_eq!(attempts, secs(&[0, 600]));
    }
}
