// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::request_filter::allowed_hosts::group::HostsGroup;
use crate::request_filter::allowed_hosts::host::Host;
use nym_task::TaskClient;
use regex::Regex;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, RwLockReadGuard};

const STANDARD_LIST_URL: &str =
    "https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt";

fn remove_comments(text: String) -> String {
    if let Ok(regex) = Regex::new(r"#.*\n") {
        regex.replace_all(&text, "").into_owned()
    } else {
        log::warn!("Failed to strip comments from standard allowed list");
        text
    }
}

/// Fetch the standard allowed list from nymtech.net
pub(crate) async fn fetch() -> Vec<Host> {
    log::info!("Refreshing standard allowed hosts");
    let text = get_standard_allowed_list().await;
    remove_comments(text)
        .split_whitespace()
        .map(Into::into)
        .collect()
}

async fn get_standard_allowed_list() -> String {
    reqwest::get(STANDARD_LIST_URL)
        .await
        .expect("failed to get allowed hosts")
        .text()
        .await
        .expect("failed to get allowed hosts text")
}

#[derive(Clone, Debug)]
pub(crate) struct StandardList {
    inner: Arc<RwLock<HostsGroup>>,
}

impl StandardList {
    // note: standard list will be fetched immediately when `StandardListUpdater::run` is called
    // (because first `tick()` of tokio interval fires up immediately)
    pub(crate) fn new() -> Self {
        StandardList {
            inner: Arc::new(RwLock::new(HostsGroup::new(Vec::new()))),
        }
    }

    pub(crate) async fn update(&self) {
        let raw_standard_list = fetch().await;
        log::debug!("fetched allowed hosts: {:?}", raw_standard_list);

        let new_data = HostsGroup::new(raw_standard_list);
        *self.inner.write().await = new_data
    }

    pub(crate) async fn get(&self) -> RwLockReadGuard<'_, HostsGroup> {
        self.inner.read().await
    }
}

pub(crate) struct StandardListUpdater {
    update_interval: Duration,
    standard_list: StandardList,

    // Listens to shutdown commands from higher up
    shutdown_listener: TaskClient,
}

impl StandardListUpdater {
    pub(crate) fn new(
        update_interval: Duration,
        standard_list: StandardList,
        shutdown_listener: TaskClient,
    ) -> Self {
        Self {
            update_interval,
            standard_list,
            shutdown_listener,
        }
    }

    pub(crate) async fn run(&mut self) {
        let mut update_interval = tokio::time::interval(self.update_interval);

        while !self.shutdown_listener.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown_listener.recv() => {
                    log::trace!("StandardListUpdater: Received shutdown");
                }
                _ = update_interval.tick() => {
                    log::debug!("updating standard list");
                    self.standard_list.update().await
                }
            }
        }

        log::debug!("StandardListUpdater: Exiting");
    }

    pub(crate) fn start(mut self) {
        tokio::spawn(async move { self.run().await });
    }
}
