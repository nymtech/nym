// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::cache::SharedCache;
use async_trait::async_trait;
use nym_task::ShutdownToken;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, trace, warn};

/// How soon a failed refresh is retried while a cache has never been built, so that a transient
/// failure at startup costs seconds rather than a whole refresh interval.
pub(crate) const FIRST_BUILD_RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// Explanation on generics:
/// the CacheItemProvider provides a value of type S, which is converted into T and swapped in for
/// the internal SharedCache<T>'s value
pub struct CacheRefresher<T, E, S = T> {
    name: String,
    refreshing_interval: Duration,

    // TODO: the Send + Sync bounds are only required for the `start_with_delay` method. could we maybe make it less restrictive?
    provider: Box<dyn CacheItemProvider<Error = E, Item = S> + Send + Sync>,
    shared_cache: SharedCache<T>,
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
// S: data type retrieved during update operation, converted into T
impl<T, E, S> CacheRefresher<T, E, S>
where
    E: std::error::Error,
    S: Into<T>,
{
    pub(crate) fn new_boxed(
        item_provider: Box<dyn CacheItemProvider<Error = E, Item = S> + Send + Sync>,
        refreshing_interval: Duration,
    ) -> Self {
        CacheRefresher {
            name: "GenericCacheRefresher".to_string(),
            refreshing_interval,
            provider: item_provider,
            shared_cache: SharedCache::new(),
        }
    }
    pub(crate) fn new<P>(item_provider: P, refreshing_interval: Duration) -> Self
    where
        P: CacheItemProvider<Error = E, Item = S> + Send + Sync + 'static,
    {
        Self::new_boxed(Box::new(item_provider), refreshing_interval)
    }

    #[must_use]
    pub(crate) fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    #[allow(dead_code)]
    pub(crate) fn get_shared_cache(&self) -> SharedCache<T> {
        self.shared_cache.clone()
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

        self.overwrite_cache(updated_items.into()).await;
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

        let mut refresh_interval = interval(self.refreshing_interval);
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    trace!("{}: Received shutdown", self.name);
                    break
                }
                _ = refresh_interval.tick() => self.refresh(&shutdown_token).await,
            }
            if self.shared_cache.get().await.is_err() {
                refresh_interval.reset_after(FIRST_BUILD_RETRY_INTERVAL);
            }
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
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

    /// When, from its start, a refresher whose refreshes go as `outcomes` attempts them during
    /// the first `elapsed`.
    async fn attempts(outcomes: &[bool], elapsed: Duration) -> Vec<Duration> {
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let provider = ScriptedProvider {
            outcomes: outcomes.iter().copied().collect(),
            attempts: attempts.clone(),
        };
        let mut refresher = CacheRefresher::<u32, RefreshFailed>::new(provider, REFRESH_INTERVAL);

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
        let attempts = attempts(&[false, false, true], Duration::from_secs(700)).await;

        assert_eq!(attempts, secs(&[0, 30, 60, 660]));
    }

    #[tokio::test(start_paused = true)]
    async fn after_the_first_build_a_failure_waits_for_the_normal_interval() {
        let attempts = attempts(&[true, false], Duration::from_secs(1300)).await;

        assert_eq!(attempts, secs(&[0, 600, 1200]));
    }
}
