// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymvisorError;
use std::env::VarError;
use std::path::PathBuf;
use std::time::Duration;

const TRUTHY_BOOLS: &[&str] = &["true", "t", "1"];
const FALSY_BOOLS: &[&str] = &["false", "f", "0"];

pub mod vars {
    pub const NYMVISOR_ID: &str = "NYMVISOR_ID";
    pub const NYMVISOR_CONFIG_PATH: &str = "NYMVISOR_CONFIG_PATH";
    pub const NYMVISOR_DATA_DIRECTORY: &str = "NYMVISOR_DATA_DIRECTORY";
    pub const NYMVISOR_DISABLE_LOGS: &str = "NYMVISOR_DISABLE_LOGS";
    pub const NYMVISOR_DATA_UPGRADE_DIRECTORY: &str = "NYMVISOR_DATA_UPGRADE_DIRECTORY";

    pub const DAEMON_NAME: &str = "DAEMON_NAME";
    pub const DAEMON_HOME: &str = "DAEMON_HOME";
    pub const DAEMON_ALLOW_BINARIES_DOWNLOAD: &str = "DAEMON_ALLOW_BINARIES_DOWNLOAD";
    pub const DAEMON_ENFORCE_DOWNLOAD_CHECKSUM: &str = "DAEMON_ENFORCE_DOWNLOAD_CHECKSUM";
    pub const DAEMON_RESTART_AFTER_UPGRADE: &str = "DAEMON_RESTART_AFTER_UPGRADE";
    pub const DAEMON_RESTART_ON_FAILURE: &str = "DAEMON_RESTART_ON_FAILURE";
    pub const DAEMON_FAILURE_RESTART_DELAY: &str = "DAEMON_FAILURE_RESTART_DELAY";
    pub const DAEMON_MAX_STARTUP_FAILURES: &str = "DAEMON_MAX_STARTUP_FAILURES";
    pub const DAEMON_STARTUP_PERIOD_DURATION: &str = "DAEMON_STARTUP_PERIOD_DURATION";
    pub const DAEMON_SHUTDOWN_GRACE_PERIOD: &str = "DAEMON_SHUTDOWN_GRACE_PERIOD";
    pub const DAEMON_DATA_BACKUP_DIRECTORY: &str = "DAEMON_DATA_BACKUP_DIRECTORY";
    pub const DAEMON_UNSAFE_SKIP_BACKUP: &str = "DAEMON_UNSAFE_SKIP_BACKUP";
}

pub(crate) fn setup_env(config_env_file: &Option<PathBuf>) -> Result<(), NymvisorError> {
    if let Some(env_file) = config_env_file {
        dotenvy::from_path_override(env_file).map_err(Into::into)
    } else {
        Ok(())
    }
}

pub(crate) struct Env {
    pub(crate) nymvisor_id: Option<String>,
    pub(crate) nymvisor_config_path: Option<PathBuf>,
    pub(crate) nymvisor_data_directory: Option<PathBuf>,
    pub(crate) nymvisor_disable_logs: Option<bool>,
    pub(crate) nymvisor_data_upgrade_directory: Option<PathBuf>,

    pub(crate) daemon_name: Option<String>,
    pub(crate) daemon_home: Option<PathBuf>,
    pub(crate) daemon_allow_binaries_download: Option<bool>,
    pub(crate) daemon_enforce_download_checksum: Option<bool>,
    pub(crate) daemon_restart_after_upgrade: Option<bool>,
    pub(crate) daemon_restart_on_failure: Option<bool>,
    pub(crate) daemon_failure_restart_delay: Option<Duration>,
    pub(crate) daemon_max_startup_failures: Option<usize>,
    pub(crate) daemon_startup_period_duration: Option<Duration>,
    pub(crate) daemon_shutdown_grace_period: Option<Duration>,
    pub(crate) daemon_data_backup_directory: Option<PathBuf>,
    pub(crate) daemon_unsafe_skip_backup: Option<bool>,
}

// TODO: all of those seem like they could be moved to some common crate if we ever needed similar functionality elsewhere
fn read_string(var: &str) -> Result<Option<String>, NymvisorError> {
    match std::env::var(var) {
        Ok(val) => Ok(Some(val)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(value)) => Err(NymvisorError::MalformedEnvVariable {
            variable: var.to_string(),
            value,
        }),
    }
}

fn read_bool(var: &str) -> Result<Option<bool>, NymvisorError> {
    read_string(var)?
        .map(|raw| {
            let normalised = raw.to_ascii_lowercase();
            if TRUTHY_BOOLS.contains(&&*normalised) {
                Ok(true)
            } else if FALSY_BOOLS.contains(&&*normalised) {
                Ok(false)
            } else {
                Err(NymvisorError::MalformedBoolEnvVariable {
                    variable: var.to_string(),
                    value: raw.to_string(),
                })
            }
        })
        .transpose()
}

fn read_duration(var: &str) -> Result<Option<Duration>, NymvisorError> {
    read_string(var)?
        .map(|raw| {
            humantime::parse_duration(&raw).map_err(|source| {
                NymvisorError::MalformedDurationEnvVariable {
                    variable: var.to_string(),
                    value: raw.to_string(),
                    source,
                }
            })
        })
        .transpose()
}

fn read_pathbuf(var: &str) -> Result<Option<PathBuf>, NymvisorError> {
    Ok(read_string(var)?.map(PathBuf::from))
}

fn read_usize(var: &str) -> Result<Option<usize>, NymvisorError> {
    read_string(var)?
        .map(|raw| {
            raw.parse()
                .map_err(|source| NymvisorError::MalformedNumberEnvVariable {
                    variable: var.to_string(),
                    value: raw.to_string(),
                    source,
                })
        })
        .transpose()
}

impl Env {
    // in general, if variable is missing from the environment that's fine.
    // however, if something is out there, it MUST BE valid
    pub(crate) fn try_read() -> Result<Self, NymvisorError> {
        Ok(Env {
            nymvisor_id: read_string(vars::NYMVISOR_ID)?,
            nymvisor_config_path: read_pathbuf(vars::NYMVISOR_CONFIG_PATH)?,
            nymvisor_data_directory: read_pathbuf(vars::NYMVISOR_DATA_DIRECTORY)?,
            nymvisor_disable_logs: read_bool(vars::NYMVISOR_DISABLE_LOGS)?,
            nymvisor_data_upgrade_directory: read_pathbuf(vars::NYMVISOR_DATA_UPGRADE_DIRECTORY)?,
            daemon_name: read_string(vars::DAEMON_NAME)?,
            daemon_home: read_pathbuf(vars::DAEMON_HOME)?,
            daemon_allow_binaries_download: read_bool(vars::DAEMON_ALLOW_BINARIES_DOWNLOAD)?,
            daemon_enforce_download_checksum: read_bool(vars::DAEMON_ENFORCE_DOWNLOAD_CHECKSUM)?,
            daemon_restart_after_upgrade: read_bool(vars::DAEMON_RESTART_AFTER_UPGRADE)?,
            daemon_restart_on_failure: read_bool(vars::DAEMON_RESTART_ON_FAILURE)?,
            daemon_failure_restart_delay: read_duration(vars::DAEMON_FAILURE_RESTART_DELAY)?,
            daemon_max_startup_failures: read_usize(vars::DAEMON_MAX_STARTUP_FAILURES)?,
            daemon_startup_period_duration: read_duration(vars::DAEMON_STARTUP_PERIOD_DURATION)?,
            daemon_shutdown_grace_period: read_duration(vars::DAEMON_SHUTDOWN_GRACE_PERIOD)?,
            daemon_data_backup_directory: read_pathbuf(vars::DAEMON_DATA_BACKUP_DIRECTORY)?,
            daemon_unsafe_skip_backup: read_bool(vars::DAEMON_UNSAFE_SKIP_BACKUP)?,
        })
    }
}
