// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::cache::SharedCache;
use async_trait::async_trait;
use nym_task::TaskClient;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, trace};

pub struct CacheRefresher<T, E> {
    name: String,
    refreshing_interval: Duration,

    // TODO: the Send + Sync bounds are only required for the `start` method. could we maybe make it less restrictive?
    provider: Box<dyn CacheItemProvider<Error = E, Item = T> + Send + Sync>,
    shared_cache: SharedCache<T>,
    // triggers: Vec<Box<dyn RefreshTriggerTrait>>,
}

#[async_trait]
pub trait CacheItemProvider {
    type Item;
    type Error: std::error::Error;

    async fn wait_until_ready(&self) {}

    async fn try_refresh(&self) -> Result<Self::Item, Self::Error>;
}

// pub struct TriggerFailure;
//
// #[async_trait]
// pub trait RefreshTriggerTrait {
//     async fn triggerred(&mut self) -> Result<(), TriggerFailure>;
// }
//
// // TODO: how to get rid of `T: Send + Sync`? it really doesn't need to be Send + Sync
// // since it's wrapped in Shared<T> internally anyway
// #[async_trait]
// impl<T> RefreshTriggerTrait for watch::Receiver<T>
// where
//     T: Send + Sync,
// {
//     async fn triggerred(&mut self) -> Result<(), TriggerFailure> {
//         self.changed().await.map_err(|err| {
//             error!("failed to process refresh trigger: {err}");
//             TriggerFailure
//         })
//     }
// }

impl<T, E> CacheRefresher<T, E>
where
    E: std::error::Error,
{
    pub(crate) fn new(
        item_provider: Box<dyn CacheItemProvider<Error = E, Item = T> + Send + Sync>,
        refreshing_interval: Duration,
    ) -> Self {
        CacheRefresher {
            name: "GenericCacheRefresher".to_string(),
            refreshing_interval,
            provider: item_provider,
            shared_cache: SharedCache::new(),
        }
    }

    pub(crate) fn new_with_initial_value(
        item_provider: Box<dyn CacheItemProvider<Error = E, Item = T> + Send + Sync>,
        refreshing_interval: Duration,
        shared_cache: SharedCache<T>,
    ) -> Self {
        CacheRefresher {
            name: "GenericCacheRefresher".to_string(),
            refreshing_interval,
            provider: item_provider,
            shared_cache,
        }
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

    // TODO: in the future offer 2 options of refreshing cache. either provide `T` directly
    // or via `FnMut(&mut T)` closure
    async fn do_refresh_cache(&self) {
        match self.provider.try_refresh().await {
            Ok(updated_items) => {
                self.shared_cache.update(updated_items).await;
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
}
