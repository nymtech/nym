// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::NymvisorError;
use std::env::VarError;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

const TRUTHY_BOOLS: &[&str] = &["true", "t", "1"];
const FALSY_BOOLS: &[&str] = &["false", "f", "0"];

pub mod vars {
    pub const NYMVISOR_ID: &str = "NYMVISOR_ID";
    pub const NYMVISOR_CONFIG_PATH: &str = "NYMVISOR_CONFIG_PATH";
    pub const NYMVISOR_UPSTREAM_BASE_UPGRADE_URL: &str = "NYMVISOR_UPSTREAM_BASE_UPGRADE_URL";
    pub const NYMVISOR_UPSTREAM_POLLING_RATE: &str = "NYMVISOR_UPSTREAM_POLLING_RATE";
    pub const NYMVISOR_DISABLE_LOGS: &str = "NYMVISOR_DISABLE_LOGS";
    pub const NYMVISOR_UPGRADE_DATA_DIRECTORY: &str = "NYMVISOR_UPGRADE_DATA_DIRECTORY";

    pub const DAEMON_NAME: &str = "DAEMON_NAME";
    pub const DAEMON_HOME: &str = "DAEMON_HOME";
    pub const DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL: &str = "DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL";
    pub const DAEMON_ALLOW_BINARIES_DOWNLOAD: &str = "DAEMON_ALLOW_BINARIES_DOWNLOAD";
    pub const DAEMON_ENFORCE_DOWNLOAD_CHECKSUM: &str = "DAEMON_ENFORCE_DOWNLOAD_CHECKSUM";
    pub const DAEMON_RESTART_AFTER_UPGRADE: &str = "DAEMON_RESTART_AFTER_UPGRADE";
    pub const DAEMON_RESTART_ON_FAILURE: &str = "DAEMON_RESTART_ON_FAILURE";
    pub const DAEMON_FAILURE_RESTART_DELAY: &str = "DAEMON_FAILURE_RESTART_DELAY";
    pub const DAEMON_MAX_STARTUP_FAILURES: &str = "DAEMON_MAX_STARTUP_FAILURES";
    pub const DAEMON_STARTUP_PERIOD_DURATION: &str = "DAEMON_STARTUP_PERIOD_DURATION";
    pub const DAEMON_SHUTDOWN_GRACE_PERIOD: &str = "DAEMON_SHUTDOWN_GRACE_PERIOD";
    pub const DAEMON_BACKUP_DATA_DIRECTORY: &str = "DAEMON_BACKUP_DATA_DIRECTORY";
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
    pub(crate) nymvisor_upstream_base_upgrade_url: Option<Url>,
    pub(crate) nymvisor_upstream_polling_rate: Option<Duration>,
    pub(crate) nymvisor_disable_logs: Option<bool>,
    pub(crate) nymvisor_upgrade_data_directory: Option<PathBuf>,

    pub(crate) daemon_name: Option<String>,
    pub(crate) daemon_home: Option<PathBuf>,
    pub(crate) daemon_absolute_upstream_upgrade_url: Option<Url>,
    pub(crate) daemon_allow_binaries_download: Option<bool>,
    pub(crate) daemon_enforce_download_checksum: Option<bool>,
    pub(crate) daemon_restart_after_upgrade: Option<bool>,
    pub(crate) daemon_restart_on_failure: Option<bool>,
    pub(crate) daemon_failure_restart_delay: Option<Duration>,
    pub(crate) daemon_max_startup_failures: Option<usize>,
    pub(crate) daemon_startup_period_duration: Option<Duration>,
    pub(crate) daemon_shutdown_grace_period: Option<Duration>,
    pub(crate) backup_data_directory: Option<PathBuf>,
    pub(crate) daemon_unsafe_skip_backup: Option<bool>,
}

impl Env {
    pub(crate) fn override_config(&self, config: &mut Config) {
        if let Some(nymvisor_id) = &self.nymvisor_id {
            config.nymvisor.id = nymvisor_id.clone();
        }
        if let Some(upstream) = &self.nymvisor_upstream_base_upgrade_url {
            config.nymvisor.debug.upstream_base_upgrade_url = upstream.clone()
        }
        if let Some(polling_rate) = self.nymvisor_upstream_polling_rate {
            config.nymvisor.debug.upstream_polling_rate = polling_rate
        }
        if let Some(nymvisor_disable_logs) = self.nymvisor_disable_logs {
            config.nymvisor.debug.disable_logs = nymvisor_disable_logs;
        }
        if let Some(nymvisor_upgrade_data_directory) = &self.nymvisor_upgrade_data_directory {
            config.nymvisor.debug.upgrade_data_directory =
                Some(nymvisor_upgrade_data_directory.clone());
        }
        if let Some(daemon_name) = &self.daemon_name {
            config.daemon.name = daemon_name.clone();
        }
        if let Some(daemon_home) = &self.daemon_home {
            config.daemon.home = daemon_home.clone();
        }
        if let Some(upstream) = &self.daemon_absolute_upstream_upgrade_url {
            config.daemon.debug.absolute_upstream_upgrade_url = Some(upstream.clone())
        }
        if let Some(daemon_allow_binaries_download) = self.daemon_allow_binaries_download {
            config.daemon.debug.allow_binaries_download = daemon_allow_binaries_download;
        }
        if let Some(daemon_enforce_download_checksum) = self.daemon_enforce_download_checksum {
            config.daemon.debug.enforce_download_checksum = daemon_enforce_download_checksum;
        }
        if let Some(daemon_restart_after_upgrade) = self.daemon_restart_after_upgrade {
            config.daemon.debug.restart_after_upgrade = daemon_restart_after_upgrade;
        }
        if let Some(daemon_restart_on_failure) = self.daemon_restart_on_failure {
            config.daemon.debug.restart_on_failure = daemon_restart_on_failure;
        }
        if let Some(daemon_failure_restart_delay) = self.daemon_failure_restart_delay {
            config.daemon.debug.failure_restart_delay = daemon_failure_restart_delay;
        }
        if let Some(daemon_max_startup_failures) = self.daemon_max_startup_failures {
            config.daemon.debug.max_startup_failures = daemon_max_startup_failures;
        }
        if let Some(daemon_startup_period_duration) = self.daemon_startup_period_duration {
            config.daemon.debug.startup_period_duration = daemon_startup_period_duration;
        }
        if let Some(daemon_shutdown_grace_period) = self.daemon_shutdown_grace_period {
            config.daemon.debug.shutdown_grace_period = daemon_shutdown_grace_period;
        }
        if let Some(backup_data_directory) = &self.backup_data_directory {
            config.daemon.debug.backup_data_directory = Some(backup_data_directory.clone());
        }
        if let Some(daemon_unsafe_skip_backup) = self.daemon_unsafe_skip_backup {
            config.daemon.debug.unsafe_skip_backup = daemon_unsafe_skip_backup;
        }
    }
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

fn read_url(var: &str) -> Result<Option<Url>, NymvisorError> {
    read_string(var)?
        .map(|raw| {
            raw.parse()
                .map_err(|source| NymvisorError::MalformedUrlEnvVariable {
                    variable: var.to_string(),
                    value: raw.to_string(),
                    source,
                })
        })
        .transpose()
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
            nymvisor_upstream_base_upgrade_url: read_url(vars::NYMVISOR_UPSTREAM_BASE_UPGRADE_URL)?,
            nymvisor_upstream_polling_rate: read_duration(vars::NYMVISOR_UPSTREAM_POLLING_RATE)?,
            nymvisor_disable_logs: read_bool(vars::NYMVISOR_DISABLE_LOGS)?,
            nymvisor_upgrade_data_directory: read_pathbuf(vars::NYMVISOR_UPGRADE_DATA_DIRECTORY)?,
            daemon_name: read_string(vars::DAEMON_NAME)?,
            daemon_home: read_pathbuf(vars::DAEMON_HOME)?,
            daemon_absolute_upstream_upgrade_url: read_url(
                vars::DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL,
            )?,
            daemon_allow_binaries_download: read_bool(vars::DAEMON_ALLOW_BINARIES_DOWNLOAD)?,
            daemon_enforce_download_checksum: read_bool(vars::DAEMON_ENFORCE_DOWNLOAD_CHECKSUM)?,
            daemon_restart_after_upgrade: read_bool(vars::DAEMON_RESTART_AFTER_UPGRADE)?,
            daemon_restart_on_failure: read_bool(vars::DAEMON_RESTART_ON_FAILURE)?,
            daemon_failure_restart_delay: read_duration(vars::DAEMON_FAILURE_RESTART_DELAY)?,
            daemon_max_startup_failures: read_usize(vars::DAEMON_MAX_STARTUP_FAILURES)?,
            daemon_startup_period_duration: read_duration(vars::DAEMON_STARTUP_PERIOD_DURATION)?,
            daemon_shutdown_grace_period: read_duration(vars::DAEMON_SHUTDOWN_GRACE_PERIOD)?,
            backup_data_directory: read_pathbuf(vars::DAEMON_BACKUP_DATA_DIRECTORY)?,
            daemon_unsafe_skip_backup: read_bool(vars::DAEMON_UNSAFE_SKIP_BACKUP)?,
        })
    }
}
