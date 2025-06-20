// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::cache::SharedCache;
use crate::support::caching::CacheNotification;
use async_trait::async_trait;
use nym_task::TaskClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Notify};
use tokio::time::interval;
use tracing::{debug, error, info, trace, warn};

pub(crate) type CacheUpdateWatcher = watch::Receiver<CacheNotification>;

#[derive(Clone)]
pub struct RefreshRequester(Arc<Notify>);

impl RefreshRequester {
    pub(crate) fn request_cache_refresh(&self) {
        self.0.notify_waiters()
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

    async fn try_refresh(&self) -> Result<Option<Self::Item>, Self::Error>;
}

impl<T, E, S> CacheRefresher<T, E, S>
where
    E: std::error::Error,
    S: Into<T>,
{
    pub(crate) fn new(
        item_provider: Box<dyn CacheItemProvider<Error = E, Item = S> + Send + Sync>,
        refreshing_interval: Duration,
    ) -> Self {
        let (refresh_notification_sender, _) = watch::channel(CacheNotification::Start);

        CacheRefresher {
            name: "GenericCacheRefresher".to_string(),
            refreshing_interval,
            refresh_notification_sender,
            update_fn: None,
            provider: item_provider,
            shared_cache: SharedCache::new(),
            refresh_requester: Default::default(),
        }
    }

    pub(crate) fn new_with_initial_value(
        item_provider: Box<dyn CacheItemProvider<Error = E, Item = S> + Send + Sync>,
        refreshing_interval: Duration,
        shared_cache: SharedCache<T>,
    ) -> Self {
        let (refresh_notification_sender, _) = watch::channel(CacheNotification::Start);

        CacheRefresher {
            name: "GenericCacheRefresher".to_string(),
            refreshing_interval,
            refresh_notification_sender,
            update_fn: None,
            provider: item_provider,
            shared_cache,
            refresh_requester: Default::default(),
        }
    }

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

    async fn do_refresh_cache(&self) {
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

        if !self.refresh_notification_sender.is_closed()
            && self
                .refresh_notification_sender
                .send(CacheNotification::Updated)
                .is_err()
        {
            warn!("failed to send cache update notification");
        }
    }

    pub async fn refresh(&self, task_client: &mut TaskClient) {
        info!("{}: refreshing cache state", self.name);

        tokio::select! {
            biased;
            _ = task_client.recv() => {
                trace!("{}: Received shutdown while refreshing cache", self.name)
            }
            _ = self.do_refresh_cache() => (),
        }
    }

    pub async fn run(&self, mut task_client: TaskClient) {
        self.provider.wait_until_ready().await;

        let mut refresh_interval = interval(self.refreshing_interval);
        while !task_client.is_shutdown() {
            tokio::select! {
                biased;
                _ = task_client.recv() => {
                    trace!("{}: Received shutdown", self.name)
                }
                _ = refresh_interval.tick() => self.refresh(&mut task_client).await,
                // note: `Notify` is not cancellation safe, HOWEVER, there's only one listener,
                // so it doesn't matter if we lose our queue position
                _ = self.refresh_requester.0.notified() => {
                    self.refresh(&mut task_client).await;
                    // since we just performed the full request, we can reset our existing interval
                    refresh_interval.reset();
                }
            }
        }
    }

    pub fn start(self, task_client: TaskClient)
    where
        T: Send + Sync + 'static,
        E: Send + Sync + 'static,
        S: Send + Sync + 'static,
    {
        tokio::spawn(async move { self.run(task_client).await });
    }

    pub fn start_with_watcher(self, task_client: TaskClient) -> CacheUpdateWatcher
    where
        T: Send + Sync + 'static,
        E: Send + Sync + 'static,
        S: Send + Sync + 'static,
    {
        let receiver = self.update_watcher();
        self.start(task_client);
        receiver
    }
}
