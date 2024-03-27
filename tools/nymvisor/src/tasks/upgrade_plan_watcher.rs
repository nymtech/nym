// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::NymvisorError;
use futures::channel::mpsc;
use nym_async_file_watcher::{AsyncFileWatcher, FileWatcherEventReceiver, NotifyResult};
use tokio::task::JoinHandle;
use tracing::warn;

pub(crate) fn start_upgrade_plan_watcher(
    config: &Config,
) -> Result<(FileWatcherEventReceiver, JoinHandle<NotifyResult<()>>), NymvisorError> {
    let (events_sender, events_receiver) = mpsc::unbounded();
    let mut watcher =
        AsyncFileWatcher::new_file_changes_watcher(config.upgrade_plan_filepath(), events_sender)?;

    let join_handle = tokio::spawn(async move {
        let res = watcher.watch().await;
        warn!("the upgrade plan watcher has stopped");
        res
    });

    Ok((events_receiver, join_handle))
}
