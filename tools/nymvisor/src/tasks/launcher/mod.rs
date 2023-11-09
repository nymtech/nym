// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{Config, DAEMON_CONFIG_DIR, DAEMON_DATA_DIR};
use crate::daemon::Daemon;
use crate::error::NymvisorError;
use crate::upgrades::{
    types::{UpgradeInfo, UpgradePlan},
    upgrade_binary,
};
use async_file_watcher::FileWatcherEventReceiver;
use flate2::write::GzEncoder;
use flate2::Compression;
use futures::future::{FusedFuture, OptionFuture};
use futures::{FutureExt, StreamExt};
use nym_task::signal::wait_for_signal;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::{format_description, OffsetDateTime};
use tokio::pin;
use tokio::sync::Notify;
use tokio::time::{sleep, Sleep};
use tracing::{debug, error, info, warn};

pub(crate) struct DaemonLauncher {
    config: Config,
    upgrade_plan_watcher: FileWatcherEventReceiver,
}

impl DaemonLauncher {
    pub(crate) fn new(config: Config) -> Self {
        todo!()
    }

    // the full upgrade process process, i.e. run until upgrade, do backup and perform the upgrade.
    // returns a boolean indicating whether the process should get restarted
    pub(crate) async fn run(&mut self, args: Vec<String>) -> Result<bool, NymvisorError> {
        let upgrade_available = self.wait_for_upgrade_or_termination(args.clone()).await?;
        if !upgrade_available {
            return Ok(false);
        }

        self.perform_backup()?;
        upgrade_binary(&self.config).await?;

        todo!()
    }

    /// this function gets called whenever the file watcher detects changes in the upgrade plan file
    /// it returns an option indicating when the next upgrade should be performed
    fn check_upgrade_plan_changes(&self) -> Option<Duration> {
        info!("checking changes in the upgrade plan file...");

        let current_upgrade_plan = match UpgradePlan::try_load(self.config.upgrade_plan_filepath())
        {
            Ok(upgrade_plan) => upgrade_plan,
            Err(err) => {
                error!("failed to read the current upgrade plan: {err}");
                return None;
            }
        };

        if let Some(next) = current_upgrade_plan.next_upgrade() {
            let now = OffsetDateTime::now_utc();
            Some((next.upgrade_time - now).try_into().unwrap_or_default())
        } else {
            None
        }
    }

    // responsible for running until exit or until update is detected
    async fn wait_for_upgrade_or_termination(
        &mut self,
        args: Vec<String>,
    ) -> Result<bool, NymvisorError> {
        let daemon = Daemon::from_config(&self.config);
        let current_upgrade = UpgradeInfo::try_load(self.config.current_upgrade_info_filepath())?;

        // see if there's already a queued up upgrade
        let current_upgrade_plan = UpgradePlan::try_load(self.config.upgrade_plan_filepath())?;
        let next = current_upgrade_plan.next_upgrade();

        // TODO: /\

        let mut running_daemon = daemon.execute_async(args)?;
        let interrupt_handle = running_daemon.interrupt_handle();

        // we need to fuse the daemon future so that we could check if it has already terminated
        let mut fused_runner = running_daemon.fuse();

        let mut upgrade_timeout: OptionFuture<_> = None.into();

        let signal_fut = wait_for_signal();
        pin!(signal_fut);

        let mut received_interrupt = false;
        loop {
            tokio::select! {
                daemon_res = &mut fused_runner => {
                    warn!("the daemon has terminated by itself - was it a short lived command?");
                    let exit_status = daemon_res?;
                    info!("it finished with the following exit status: {exit_status}");
                    return Ok(false)
                }
                event = &mut self.upgrade_plan_watcher.next() => {
                    let Some(event) = event else {
                        // this is a critical failure since the file watcher task should NEVER terminate by itself
                        error!("CRITICAL FAILURE: the upgrade plan watcher channel got closed");
                        panic!("CRITICAL FAILURE: the upgrade plan watcher channel got closed")
                    };
                    println!("the file has changed - {event:?}");

                    debug!("the file has changed - {event:?}");
                    if let Some(next_upgrade) = self.check_upgrade_plan_changes() {
                        info!("setting the upgrade timeout to {}", humantime::format_duration(next_upgrade));
                        upgrade_timeout = Some(Box::pin(sleep(next_upgrade)).fuse()).into()
                    }

                }
                _ = &mut upgrade_timeout, if !upgrade_timeout.is_terminated() => {
                    info!("the upgrade timeout has elapsed. the daemon will be now stopped in order to perform the upgrade");
                    break
                }
                _ = &mut signal_fut => {
                    received_interrupt = true;
                    info!("the nymvisor has received an interrupt. the daemon will be now stopped before exiting");
                    break
                }
            }
        }

        if fused_runner.is_terminated() {
            return Ok(false);
        }
        interrupt_handle.interrupt_daemon();

        match fused_runner.await {
            Ok(exit_status) => {
                info!("the daemon finished with the following exit status: {exit_status}");
            }
            Err(err) => {
                warn!("the daemon finished with an error: {err}");
            }
        }

        // if we received an interrupt, don't try to perform upgrade, just exit the nymvisor
        Ok(!received_interrupt)
    }

    fn perform_backup(&self) -> Result<(), NymvisorError> {
        // safety: this expect is fine as we're using a constant formatter.
        #[allow(clippy::expect_used)]
        let format = format_description::parse(
            "[year]-[month]-[day]-[hour][minute][second][subsecond digits:3]",
        )
        .expect("our time formatter is malformed");
        #[allow(clippy::expect_used)]
        let now = OffsetDateTime::now_utc()
            .format(&format)
            .expect("our time formatter failed to format the current time");

        let backup_filepath = self
            .config
            .daemon_backup_dir()
            .join(format!("backup-{now}-preupgrade.tar.gz"));

        // create the backup file
        let backup_file = fs::File::create(&backup_filepath).map_err(|source| {
            NymvisorError::BackupFileCreationFailure {
                path: backup_filepath.clone(),
                source,
            }
        })?;

        let daemon_data = self.config.daemon_data_dir();
        let daemon_config = self.config.daemon_config_dir();
        let enc = GzEncoder::new(backup_file, Compression::default());
        let mut tar = tar::Builder::new(enc);
        tar.append_dir_all(DAEMON_DATA_DIR, &daemon_data)
            .map_err(|source| NymvisorError::BackupTarFailure {
                path: backup_filepath.clone(),
                data_source: daemon_data,
                source,
            })?;

        tar.append_dir_all(DAEMON_CONFIG_DIR, &daemon_config)
            .map_err(|source| NymvisorError::BackupTarFailure {
                path: backup_filepath.clone(),
                data_source: daemon_config,
                source,
            })?;

        tar.finish()
            .map_err(|source| NymvisorError::BackupTarFinalizationFailure {
                path: backup_filepath,
                source,
            })
    }
}
