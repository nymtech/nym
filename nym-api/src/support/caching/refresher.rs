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
use tracing::{error, info, trace, warn};

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

pub struct CacheRefresher<T, E> {
    name: String,
    refreshing_interval: Duration,
    refresh_notification_sender: watch::Sender<CacheNotification>,

    // TODO: the Send + Sync bounds are only required for the `start` method. could we maybe make it less restrictive?
    provider: Box<dyn CacheItemProvider<Error = E, Item = T> + Send + Sync>,
    shared_cache: SharedCache<T>,
    refresh_requester: RefreshRequester,
}

#[async_trait]
pub(crate) trait CacheItemProvider {
    type Item;
    type Error: std::error::Error;

    async fn wait_until_ready(&self) {}

    async fn try_refresh(&self) -> Result<Self::Item, Self::Error>;
}

impl<T, E> CacheRefresher<T, E>
where
    E: std::error::Error,
{
    pub(crate) fn new(
        item_provider: Box<dyn CacheItemProvider<Error = E, Item = T> + Send + Sync>,
        refreshing_interval: Duration,
    ) -> Self {
        let (refresh_notification_sender, _) = watch::channel(CacheNotification::Start);

        CacheRefresher {
            name: "GenericCacheRefresher".to_string(),
            refreshing_interval,
            refresh_notification_sender,
            provider: item_provider,
            shared_cache: SharedCache::new(),
            refresh_requester: Default::default(),
        }
    }

    pub(crate) fn new_with_initial_value(
        item_provider: Box<dyn CacheItemProvider<Error = E, Item = T> + Send + Sync>,
        refreshing_interval: Duration,
        shared_cache: SharedCache<T>,
    ) -> Self {
        let (refresh_notification_sender, _) = watch::channel(CacheNotification::Start);

        CacheRefresher {
            name: "GenericCacheRefresher".to_string(),
            refreshing_interval,
            refresh_notification_sender,
            provider: item_provider,
            shared_cache,
            refresh_requester: Default::default(),
        }
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

    // TODO: in the future offer 2 options of refreshing cache. either provide `T` directly
    // or via `FnMut(&mut T)` closure
    async fn do_refresh_cache(&self) {
        match self.provider.try_refresh().await {
            Ok(updated_items) => {
                self.shared_cache.update(updated_items).await;
                if !self.refresh_notification_sender.is_closed()
                    && self
                        .refresh_notification_sender
                        .send(CacheNotification::Updated)
                        .is_err()
                {
                    warn!("failed to send cache update notification");
                }
            }
            Err(err) => {
                error!("{}: failed to refresh the cache: {err}", self.name)
            }
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
    {
        tokio::spawn(async move { self.run(task_client).await });
    }

    pub fn start_with_watcher(self, task_client: TaskClient) -> CacheUpdateWatcher
    where
        T: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        let receiver = self.update_watcher();
        self.start(task_client);
        receiver
    }
}
