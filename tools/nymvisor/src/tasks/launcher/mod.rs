// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::daemon::Daemon;
use crate::error::NymvisorError;
use crate::upgrades::{UpgradeInfo, UpgradePlan};
use async_file_watcher::FileWatcherEventReceiver;
use futures::future::{FusedFuture, OptionFuture};
use futures::{FutureExt, StreamExt};
use nym_task::signal::wait_for_signal;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::Sleep;
use tracing::{error, info, warn};

pub(crate) struct DaemonLauncher {
    config: Config,
    upgrade_plan_watcher: FileWatcherEventReceiver,
}

impl DaemonLauncher {
    pub(crate) fn new(config: Config) -> Self {
        todo!()
    }

    // responsible for running until exit or until update is detected
    pub(crate) async fn run(&mut self, args: Vec<String>) -> Result<(), NymvisorError> {
        todo!()
    }

    fn check_upgrade_plan_changes(&self) {
        //
    }

    async fn wait_for_upgrade_or_termination(
        &mut self,
        args: Vec<String>,
    ) -> Result<bool, NymvisorError> {
        let daemon = Daemon::from_config(&self.config);
        let current_upgrade = UpgradeInfo::try_load(self.config.current_upgrade_info_filepath())?;

        // see if there's already a queued up upgrade
        let current_upgrade_plan = UpgradePlan::try_load(self.config.upgrade_plan_filepath())?;
        let next = current_upgrade_plan.next_upgrade();

        let mut running_daemon = daemon.execute_async(args)?;
        let interrupt_handle = running_daemon.interrupt_handle();

        // we need to fuse the daemon future so that we could check if it has already terminated
        let mut fused_runner = running_daemon.fuse();

        let mut upgrade_timeout: OptionFuture<_> = None.into();

        upgrade_timeout =
            Some(Box::pin(tokio::time::sleep(Duration::from_secs(123))).fuse()).into();

        let sig_fut = wait_for_signal();

        // note: this has to be in a loop because `upgrade_plan_watcher` might receive events that do not necessarily trigger the upgrade
        loop {
            tokio::select! {
                daemon_res = &mut fused_runner => {
                    warn!("the daemon has terminated by itself - was it a short lived command?");
                    let exit_status = daemon_res?;
                    info!("it finished with the following exit status: {exit_status}");
                    return Ok(false)
                }
                _ = &mut self.upgrade_plan_watcher.next() => {
                    //
                }
                _ = &mut upgrade_timeout, if !upgrade_timeout.is_terminated() => {
                    break
                }



            }
        }

        // if the runner hasn't terminated by itself (which should be the almost every single time!)
        // send the interrupt and wait for it to be done
        if fused_runner.is_terminated() {
            todo!("error case")
        }
        interrupt_handle.interrupt_daemon();
        let res = fused_runner.await;

        /*

           tokio select on:
           - daemon terminating
           - upgrade-plan.json changes
           - https://nymtech.net/.wellknown/<DAEMON_NAME>/update-info.json changes


           // todo: maybe move to a higher layer
           - signals received (to propagate them to daemon before terminating to prevent creating zombie processes)

        */

        todo!()
    }

    async fn perform_backup(&self) {
        todo!()
    }
}
