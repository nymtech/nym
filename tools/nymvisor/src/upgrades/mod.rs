// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::daemon::Daemon;
use crate::error::NymvisorError;
use crate::upgrades::download::download_upgrade_binary;
use crate::upgrades::types::{CurrentVersionInfo, UpgradeHistory, UpgradePlan};
use nix::fcntl::{flock, FlockArg};
use std::fs;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use time::OffsetDateTime;
use tracing::{debug, info};

pub(crate) mod download;
mod serde_helpers;
pub(crate) mod types;

#[derive(Default)]
pub(crate) struct UpgradeResult {
    pub(crate) binary_swapped: bool,
    pub(crate) requires_manual_intervention: bool,
}

impl UpgradeResult {
    pub(crate) fn new_shortlived() -> UpgradeResult {
        UpgradeResult {
            binary_swapped: false,
            requires_manual_intervention: false,
        }
    }
}

pub(crate) async fn perform_upgrade(config: &Config) -> Result<UpgradeResult, NymvisorError> {
    info!("attempting to perform binary upgrade");

    let mut plan = UpgradePlan::try_load(config.upgrade_plan_filepath())?;
    let Some(next) = plan.pop_next_upgrade() else {
        return Err(NymvisorError::NoQueuedUpgrades);
    };

    let requires_manual_intervention = next.manual;
    let upgrade_name = next.name.clone();

    let history_path = config.upgrade_history_filepath();
    let mut upgrade_history = if history_path.exists() {
        UpgradeHistory::try_load(history_path)?
    } else {
        UpgradeHistory::new(history_path)
    };

    debug!("creating the lock file");
    let lock_path = config.upgrade_lock_filepath();
    let lock_file =
        File::create(&lock_path).map_err(|source| NymvisorError::LockFileCreationFailure {
            path: lock_path.clone(),
            source,
        })?;
    let lock_fd = lock_file.as_raw_fd();

    debug!("attempting to acquire the lock");
    if let Err(err) = flock(lock_fd, FlockArg::LockExclusiveNonblock) {
        return Err(NymvisorError::UnableToAcquireUpgradePlanLock {
            lock_path,
            libc_code: err,
        });
    }

    let upgrade_binary_path = config.upgrade_binary(&upgrade_name);

    if !upgrade_binary_path.exists() {
        if !config.daemon.debug.allow_binaries_download {
            return Err(NymvisorError::NoUpgradeBinaryWithDisabledDownload {
                path: upgrade_binary_path,
            });
        }
        info!(
            "upgrade binary not found at '{}'. attempting to to download it",
            upgrade_binary_path.display()
        );

        download_upgrade_binary(config, &next).await?;
    }

    let tmp_daemon = Daemon::new(upgrade_binary_path);
    tmp_daemon.verify_binary()?;

    let new_bin_info = tmp_daemon.get_build_information()?;
    next.ensure_matches_bin_info(&new_bin_info)?;

    // update the 'current-version-history.json'
    CurrentVersionInfo {
        name: next.name.clone(),
        version: next.version.clone(),
        upgrade_time: OffsetDateTime::now_utc(),
        binary_details: new_bin_info,
    }
    .save(config.current_daemon_version_filepath())?;

    // update the 'upgrade-plan.json'
    plan.set_current(next.clone());
    plan.update_on_disk()?;

    // update the 'upgrade-history.json'
    upgrade_history.insert_new_upgrade(next)?;

    // update the 'current' symlink
    set_upgrade_link(config, config.upgrade_dir(&upgrade_name))?;

    // finally remove the lock file
    fs::remove_file(&lock_path).map_err(|source| NymvisorError::LockFileRemovalFailure {
        path: lock_path.clone(),
        source,
    })?;

    Ok(UpgradeResult {
        binary_swapped: true,
        requires_manual_intervention,
    })
}

fn set_upgrade_link(config: &Config, upgrade_path: PathBuf) -> Result<(), NymvisorError> {
    // remove the existing symlink if it exists
    let link = config.current_daemon_dir();
    if fs::read_link(&link).is_ok() {
        fs::remove_file(&link).map_err(|source| NymvisorError::SymlinkRemovalFailure {
            path: link.clone(),
            source,
        })?;
    }

    std::os::unix::fs::symlink(&upgrade_path, &link).map_err(|source| {
        NymvisorError::SymlinkCreationFailure {
            source_path: upgrade_path,
            target_path: link,
            source,
        }
    })
}
