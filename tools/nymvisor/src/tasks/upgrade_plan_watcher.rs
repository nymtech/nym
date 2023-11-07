// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::NymvisorError;
use crate::helpers::TaskHandle;
use async_file_watcher::{AsyncFileWatcher, FileWatcherEventReceiver, NotifyResult};
use futures::channel::mpsc;
use futures::future::{AbortHandle, Abortable};

pub(crate) fn start_upgrade_plan_watcher(
    config: &Config,
) -> Result<(FileWatcherEventReceiver, TaskHandle<NotifyResult<()>>), NymvisorError> {
    let (events_sender, events_receiver) = mpsc::unbounded();
    let mut watcher =
        AsyncFileWatcher::new_file_changes_watcher(config.upgrade_plan_filepath(), events_sender)?;

    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let join_handle =
        tokio::spawn(async move { Abortable::new(watcher.watch(), abort_registration).await });

    let task_handle = TaskHandle::new(abort_handle, join_handle);

    Ok((events_receiver, task_handle))
}
