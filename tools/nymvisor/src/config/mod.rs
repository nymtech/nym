// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::CONFIG_TEMPLATE;
use nym_config::serde_helpers::de_maybe_path;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, warn};

mod template;

const DEFAULT_FAILURE_RESTART_DELAY: Duration = Duration::from_secs(10);
const DEFAULT_STARTUP_PERIOD: Duration = Duration::from_secs(120);
const DEFAULT_MAX_STARTUP_FAILURES: usize = 10;
const DEFAULT_SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(10);

const DEFAULT_NYMVISORS_DIR: &str = "nymvisors";
const DEFAULT_NYMVISORS_INSTANCES_DIR: &str = "instances";

/// Derive default path to the nymvisor's config directory.
/// It should get resolved to `$HOME/.nym/nymvisor/instances/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYMVISORS_DIR)
        .join(DEFAULT_NYMVISORS_INSTANCES_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to the nymvisor's config file.
/// It should get resolved to `$HOME/.nym/nymvisor/instances/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to the nymvisor's data directory where additional files are stored.
/// It should get resolved to `$HOME/.nym/nymvisor/instances/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYMVISORS_DIR)
        .join(DEFAULT_NYMVISORS_INSTANCES_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

/// Get default path to nymvisors global data directory where files, such as upgrade plans or binaries are stored.
/// It should get resolved to `$HOME/.nym/nymvisors/data`
pub fn default_global_data_directory() -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYMVISORS_DIR)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    pub nymvisor: Nymvisor,

    pub daemon: Daemon,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn new() -> Self {
        todo!()
    }

    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref();
        let mut loaded: Config = read_config_from_toml_file(path)?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::read_from_path(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_path(default_config_filepath(id))
    }

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.nymvisor.id)
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    pub fn try_save(&self) -> io::Result<()> {
        if let Some(save_location) = &self.save_path {
            save_formatted_config_to_file(self, save_location)
        } else {
            warn!("config file save location is unknown. falling back to the default");
            self.save_to_default_location()
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Nymvisor {
    /// ID specifies the human readable ID of this particular nymvisor instance.
    /// Can be overridden with $NYMVISOR_ID environmental variable.
    pub id: String,

    /// Further optional configuration options associated with the nymvisor.
    pub debug: NymvisorDebug,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NymvisorDebug {
    /// If set to true, this will disable `nymvisor` logs (but not the underlying process)
    /// default: false
    /// Can be overridden with $NYMVISOR_DISABLE_LOGS environmental variable.
    pub disable_logs: bool,

    /// Set custom directory for upgrade data - binaries and upgrade plans.
    /// If not set, the global nymvisors' data directory will be used instead.
    /// Can be overridden with $NYMVISOR_DATA_UPGRADE_DIRECTORY environmental variable.
    #[serde(deserialize_with = "de_maybe_path")]
    pub data_upgrade_directory: Option<PathBuf>,
}

#[allow(clippy::derivable_impls)]
impl Default for NymvisorDebug {
    fn default() -> Self {
        NymvisorDebug {
            disable_logs: false,
            data_upgrade_directory: None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Daemon {
    /// The name of the managed binary itself (e.g. nym-api, nym-mixnode, nym-gateway, etc.)
    /// Can be overridden with $DAEMON_NAME environmental variable.
    pub name: String,

    /// The location where the `nymvisor/` directory is kept that contains the auxiliary files associated
    /// with the underlying daemon, such as any backups or current version information.
    /// (e.g. $HOME/.nym/nym-api/my-nym-api, $HOME/.nym/mixnodes/my-mixnode, etc.).
    /// Can be overridden with $DAEMON_HOME environmental variable.
    pub home: String,

    /// Further optional configuration options associated with the daemon.
    pub debug: DaemonDebug,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonDebug {
    /// If set to true, this will enable auto-downloading of new binaries using the url provided in the `upgrade-info.json`
    /// default: true
    /// Can be overridden with $DAEMON_ALLOW_BINARIES_DOWNLOAD environmental variable.
    pub allow_binaries_download: bool,

    /// If enabled nymvisor will require that a checksum is provided in the upgrade plan for the binary to be downloaded.
    /// If disabled, nymvisor will not require a checksum to be provided, but still check the checksum if one is provided.
    /// default: true
    /// Can be overridden with $DAEMON_ENFORCE_DOWNLOAD_CHECKSUM environmental variable.
    pub enforce_download_checksum: bool,

    /// If enabled, nymvisor will restart the subprocess with the same command-line arguments and flags (but with the new binary) after a successful upgrade.
    /// Otherwise (if disabled), nymvisor will stop running after an upgrade and will require the system administrator to manually restart it.
    /// Note restart is only after the upgrade and does not auto-restart the subprocess after an error occurs.
    /// default: true
    /// Can be overridden with $DAEMON_RESTART_AFTER_UPGRADE environmental variable.
    pub restart_after_upgrade: bool,

    /// If enabled, nymvisor will restart the subprocess with the same command-line arguments and flags after it has crashed
    /// default: false
    /// Can be overridden with $DAEMON_RESTART_ON_FAILURE environmental variable.
    pub restart_on_failure: bool,

    /// If `restart_on_failure` is enabled, the following value defines the amount of time `nymvisor` shall wait before
    /// restarting the subprocess.
    /// default: 10s
    /// Can be overridden with $DAEMON_FAILURE_RESTART_DELAY environmental variable.
    // The default value is so relatively high as to prevent constant restart loops in case of some underlying issue.
    #[serde(with = "humantime_serde")]
    pub failure_restart_delay: Duration,

    /// Defines the maximum number of startup failures the subprocess can experience in a quick succession before
    /// no further restarts will be attempted and `nymvisor` will exit/
    /// default: 10
    /// Can be overridden with $DAEMON_MAX_STARTUP_FAILURES environmental variable.
    pub max_startup_failures: usize,

    /// Defines the length of time during which the subprocess is still considered to be in the startup phase
    /// when its failures are going to be considered in `max_startup_failures`.
    /// default: 120s
    /// Can be overridden with $DAEMON_STARTUP_PERIOD_DURATION environmental variable.
    #[serde(with = "humantime_serde")]
    pub startup_period_duration: Duration,

    /// Specifies the amount of time `nymvisor` is willing to wait for the subprocess to undergo graceful shutdown after receiving an interrupt
    /// (for either an upgrade or shutdown of the `nymvisor` itself)
    /// Once the time passes, a kill signal is going to be sent instead.
    /// default: 10s
    /// Can be overridden with $DAEMON_SHUTDOWN_GRACE_PERIOD environmental variable.
    #[serde(with = "humantime_serde")]
    pub shutdown_grace_period: Duration,

    /// Set custom backup directory for daemon data. If not set, the daemon's home directory will be used instead.
    /// Can be overridden with $DAEMON_DATA_BACKUP_DIRECTORYenvironmental variable.
    #[serde(deserialize_with = "de_maybe_path")]
    pub data_backup_directory: Option<PathBuf>,

    /// If enabled, `nymvisor` will perform upgrades directly without performing any backups.
    /// default: false
    /// Can be overridden with $DAEMON_UNSAFE_SKIP_BACKUP environmental variable.
    pub unsafe_skip_backup: bool,
}

impl Default for DaemonDebug {
    fn default() -> Self {
        DaemonDebug {
            allow_binaries_download: true,
            enforce_download_checksum: true,
            restart_after_upgrade: true,
            restart_on_failure: false,
            failure_restart_delay: DEFAULT_FAILURE_RESTART_DELAY,
            max_startup_failures: DEFAULT_MAX_STARTUP_FAILURES,
            startup_period_duration: DEFAULT_STARTUP_PERIOD,
            shutdown_grace_period: DEFAULT_SHUTDOWN_GRACE_PERIOD,
            data_backup_directory: None,
            unsafe_skip_backup: false,
        }
    }
}
