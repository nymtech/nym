// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::daemon::Daemon;
use crate::error::NymvisorError;
use crate::tasks::launcher::backup::BackupBuilder;
use crate::upgrades::types::{CurrentVersionInfo, UpgradeInfo};
use crate::upgrades::{perform_upgrade, types::UpgradePlan, UpgradeResult};
use futures::future::{FusedFuture, OptionFuture};
use futures::{FutureExt, StreamExt};
use nym_async_file_watcher::FileWatcherEventReceiver;
use nym_task::signal::wait_for_signal;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::pin;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

mod backup;

pub(crate) struct DaemonLauncher {
    config: Config,
    upgrade_plan_watcher: FileWatcherEventReceiver,
}

impl DaemonLauncher {
    pub(crate) fn new(config: Config, upgrade_plan_watcher: FileWatcherEventReceiver) -> Self {
        DaemonLauncher {
            config,
            upgrade_plan_watcher,
        }
    }

    pub(crate) async fn run_loop(&mut self, args: Vec<String>) -> Result<(), NymvisorError> {
        let mut startup_failures = 0;
        loop {
            let run_start = tokio::time::Instant::now();

            let res = self.run_and_upgrade(args.clone()).await;
            let run_duration = run_start.elapsed();
            info!(
                "the daemon has run for {}",
                humantime::format_duration(run_duration)
            );

            match res {
                Ok(upgrade_result) => {
                    if upgrade_result.requires_manual_intervention {
                        info!("this upgrade requires manual intervention. Please read the release notes carefully and follow the provided instructions before starting nymvisor again");
                        return Ok(());
                    }

                    if upgrade_result.binary_swapped {
                        if !self.config.daemon.debug.restart_after_upgrade {
                            info!("upgrade detected, DAEMON_RESTART_AFTER_UPGRADE is off. Verify new upgrade and start nymvisor again");
                            return Ok(());
                        }
                        // else - binary has been swapped and restarting is enabled: do restart
                    } else {
                        // binary has finished its execution (short-lived process) without upgrades
                        return Ok(());
                    }
                }
                Err(failure) => {
                    error!("daemon failed with the following error: {failure}");

                    if !self.config.daemon.debug.restart_on_failure {
                        return Err(NymvisorError::DisabledRestartOnFailure);
                    }

                    if run_duration < self.config.daemon.debug.startup_period_duration {
                        startup_failures += 1;
                    } else {
                        startup_failures = 1;
                    }

                    if startup_failures >= self.config.daemon.debug.max_startup_failures {
                        return Err(NymvisorError::DaemonMaximumStartupFailures {
                            failures: startup_failures,
                        });
                    }

                    info!(
                        "waiting for {} before attempting to restart the daemon...",
                        humantime::format_duration(self.config.daemon.debug.failure_restart_delay)
                    );
                    sleep(self.config.daemon.debug.failure_restart_delay).await;
                    // restart
                }
            }
            info!("the daemon will be now restarted")
        }
    }

    /// the full upgrade process process, i.e. run until upgrade, do backup and perform the upgrade.
    /// returns a boolean indicating whether an upgrade has been performed
    async fn run_and_upgrade(&mut self, args: Vec<String>) -> Result<UpgradeResult, NymvisorError> {
        let upgrade_available = self.wait_for_upgrade_or_termination(args.clone()).await?;
        if !upgrade_available {
            return Ok(UpgradeResult::new_shortlived());
        }

        if !self.config.daemon.debug.unsafe_skip_backup {
            self.perform_backup()?;
        }

        perform_upgrade(&self.config).await
        // if we ever wanted to introduce any pre-upgrade scripts like cosmovisor, they'd go here
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

        let current_info = UpgradeInfo::try_load(self.config.current_upgrade_info_filepath())?;
        let expected_version =
            CurrentVersionInfo::try_load(self.config.current_daemon_version_filepath())?;
        let daemon_info = daemon.get_build_information()?;

        current_info.ensure_matches(&expected_version)?;
        if expected_version.binary_details != daemon_info {
            return Err(NymvisorError::UnexpectedDaemonBuild {
                daemon_info: Box::new(daemon_info),
                stored_info: Box::new(expected_version.binary_details),
            });
        }

        let mut running_daemon = daemon.execute_async(args)?;
        let interrupt_handle = running_daemon.interrupt_handle();

        // we need to fuse the daemon future so that we could check if it has already terminated
        let mut fused_runner = running_daemon.fuse();

        let mut upgrade_timeout: OptionFuture<_> = self
            .check_upgrade_plan_changes()
            .map(sleep)
            .map(Box::pin)
            .map(FutureExt::fuse)
            .into();

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
        let plan = UpgradePlan::try_load(self.config.upgrade_plan_filepath())?;

        let Some(upgrade_name) = plan.next_upgrade().map(|u| &u.name) else {
            // this should NEVER be possible, but because those famous last words have been said before,
            // let's just return an error when it inevitably happens
            return Err(NymvisorError::NoQueuedUpgrades);
        };

        BackupBuilder::new(self.config.daemon_upgrade_backup_dir(upgrade_name))?
            .backup_daemon_home(&self.config.daemon.home)
    }
}
