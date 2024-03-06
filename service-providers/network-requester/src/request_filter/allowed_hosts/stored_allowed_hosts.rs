// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::request_filter::allowed_hosts::HostsStore;
use futures::channel::mpsc;
use futures::StreamExt;
use nym_async_file_watcher::{AsyncFileWatcher, FileWatcherEventReceiver};
use nym_task::TaskClient;
use std::io;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Debug, Clone)]
pub(crate) struct StoredAllowedHosts {
    inner: Arc<RwLock<HostsStore>>,
}

impl StoredAllowedHosts {
    pub(crate) fn new<P: AsRef<Path>>(path: P) -> Self {
        let allowed_hosts = HostsStore::new(path);

        StoredAllowedHosts {
            inner: Arc::new(RwLock::new(allowed_hosts)),
        }
    }

    pub(crate) async fn reload(&self) -> io::Result<()> {
        log::debug!("reloading stored allowed hosts");
        self.inner.write().await.try_reload()
    }

    pub(crate) async fn get(&self) -> RwLockReadGuard<'_, HostsStore> {
        self.inner.read().await
    }
}

impl From<HostsStore> for StoredAllowedHosts {
    fn from(value: HostsStore) -> Self {
        StoredAllowedHosts {
            inner: Arc::new(RwLock::new(value)),
        }
    }
}

pub(crate) struct StoredAllowedHostsReloader {
    stored_hosts: StoredAllowedHosts,
    events_receiver: FileWatcherEventReceiver,

    // Listens to shutdown commands from higher up
    shutdown_listener: TaskClient,
}

impl StoredAllowedHostsReloader {
    pub(crate) fn new(
        stored_hosts: StoredAllowedHosts,
        events_receiver: FileWatcherEventReceiver,
        shutdown_listener: TaskClient,
    ) -> Self {
        StoredAllowedHostsReloader {
            events_receiver,
            stored_hosts,
            shutdown_listener,
        }
    }

    pub(crate) async fn run(&mut self) {
        while !self.shutdown_listener.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown_listener.recv() => {
                    log::trace!("StoredAllowedHostsReloader: Received shutdown");
                }
                event = self.events_receiver.next() => {
                    let Some(event) = event else {
                        log::trace!("StoredAllowedHostsReloader: sender channel has terminated");
                        break
                    };
                    log::debug!("the file has changed - {event:?}");
                    log::debug!("reloading stored hosts");
                    if let Err(err) = self.stored_hosts.reload().await {
                        log::error!("failed to reload stored hosts: {err}")
                    }
                }
            }
        }

        log::debug!("StoredAllowedHostsReloader: Exiting");
    }

    pub(crate) fn start(mut self) {
        tokio::spawn(async move { self.run().await });
    }
}

async fn run_watcher(mut watcher: AsyncFileWatcher, mut shutdown: TaskClient) {
    tokio::select! {
        biased;
        _ = shutdown.recv() => {
            log::trace!("AsyncFileWatcher: Received shutdown");
        }
        res = watcher.watch() => {
            log::trace!("AsyncFileWatcher: finished with {res:?}");
        }
    }
    log::debug!("AsyncFileWatcher: Exiting");
}

fn start_watcher(watcher: AsyncFileWatcher, shutdown: TaskClient) {
    tokio::spawn(async move { run_watcher(watcher, shutdown).await });
}

pub(crate) async fn start_allowed_list_reloader(
    stored_list: StoredAllowedHosts,
    shutdown_listener: TaskClient,
) {
    let (events_sender, events_receiver) = mpsc::unbounded();
    let file = stored_list.get().await.storefile.clone();

    let watcher = AsyncFileWatcher::new_file_changes_watcher(file, events_sender)
        .expect("failed to create file watcher");
    let reloader =
        StoredAllowedHostsReloader::new(stored_list, events_receiver, shutdown_listener.clone());

    start_watcher(watcher, shutdown_listener);
    reloader.start()
}
