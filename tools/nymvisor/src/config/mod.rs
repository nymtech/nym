// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::CONFIG_TEMPLATE;
use nym_config::serde_helpers::de_maybe_stringified;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, warn};
use url::Url;

mod template;

pub(crate) const DEFAULT_FAILURE_RESTART_DELAY: Duration = Duration::from_secs(10);
pub(crate) const DEFAULT_STARTUP_PERIOD: Duration = Duration::from_secs(120);
pub(crate) const DEFAULT_MAX_STARTUP_FAILURES: usize = 10;
pub(crate) const DEFAULT_SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(10);
pub(crate) const DEFAULT_UPSTREAM_POLLING_RATE: Duration = Duration::from_secs(60 * 60);

pub(crate) const DEFAULT_BASE_UPSTREAM_UPGRADE_INFO_SOURCE: &str =
    "https://nymtech.net/.wellknown/";

pub(crate) const UPGRADE_PLAN_FILENAME: &str = "upgrade-plan.json";
pub(crate) const UPGRADE_HISTORY_FILENAME: &str = "upgrade-history.json";
pub(crate) const UPGRADE_LOCK_FILENAME: &str = "upgrade.lock";
pub(crate) const UPGRADE_INFO_FILENAME: &str = "upgrade-info.json";
pub(crate) const CURRENT_VERSION_FILENAME: &str = "current-version.json";
pub(crate) const NYMVISOR_DIR: &str = "nymvisor";
pub(crate) const BACKUP_DIR: &str = "backups";
pub(crate) const GENESIS_DIR: &str = "genesis";
pub(crate) const CURRENT_DIR: &str = "current";
pub(crate) const BIN_DIR: &str = "bin";
pub(crate) const UPGRADES_DIR: &str = "upgrades";
pub(crate) const DEFAULT_NYMVISORS_DIR: &str = "nymvisors";
pub(crate) const DEFAULT_NYMVISORS_INSTANCES_DIR: &str = "instances";

/// Derive default path top the nymvisors instance directory.
/// It should get resolved to `$HOME/.nym/nymvisor/instances`
pub fn default_instances_directory() -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYMVISORS_DIR)
        .join(DEFAULT_NYMVISORS_INSTANCES_DIR)
}

/// Derive default path to the nymvisor's config directory.
/// It should get resolved to `$HOME/.nym/nymvisor/instances/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    default_instances_directory()
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to the nymvisor's config file.
/// It should get resolved to `$HOME/.nym/nymvisor/instances/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

// /// Derive default path to the nymvisor's data directory where additional files are stored.
// /// It should get resolved to `$HOME/.nym/nymvisor/instances/<id>/data`
// pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
//     must_get_home()
//         .join(NYM_DIR)
//         .join(DEFAULT_NYMVISORS_DIR)
//         .join(DEFAULT_NYMVISORS_INSTANCES_DIR)
//         .join(id)
//         .join(DEFAULT_DATA_DIR)
// }

/// Get default path to nymvisors global data directory where files, such as upgrade plans or binaries are stored.
/// It should get resolved to `$HOME/.nym/nymvisors/data`
pub fn default_global_data_directory() -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYMVISORS_DIR)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
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

impl Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"
{:<35}{}
{:<35}{}
{:<35}{}
{:<35}{}
{:<35}{}
{:<35}{:?}
{:<35}{:?}
{:<35}{}
{:<35}{}
{:<35}{}
{:<35}{}
{:<35}{}
{:<35}{}
{:<35}{}
{:<35}{}
{:<35}{:?}
{:<35}{}
"#,
            "id:",
            self.nymvisor.id,
            "daemon name:",
            self.daemon.name,
            "daemon home:",
            self.daemon.home.display(),
            "upstream base upgrade url:",
            self.nymvisor.debug.upstream_base_upgrade_url,
            "disable nymvisor logs:",
            self.nymvisor.debug.disable_logs,
            "CUSTOM upgrade data directory",
            self.nymvisor
                .debug
                .upgrade_data_directory
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            "upstream absolute upgrade url:",
            self.daemon
                .debug
                .absolute_upstream_upgrade_url
                .as_ref()
                .map(|p| p.to_string())
                .unwrap_or_default(),
            "allow binaries download:",
            self.daemon.debug.allow_binaries_download,
            "enforce download checksum:",
            self.daemon.debug.enforce_download_checksum,
            "restart after upgrade:",
            self.daemon.debug.restart_after_upgrade,
            "restart on failure:",
            self.daemon.debug.restart_on_failure,
            "on failure restart delay:",
            humantime::format_duration(self.daemon.debug.failure_restart_delay),
            "max startup failures:",
            self.daemon.debug.max_startup_failures,
            "startup period duration:",
            humantime::format_duration(self.daemon.debug.startup_period_duration),
            "shutdown grace period:",
            humantime::format_duration(self.daemon.debug.shutdown_grace_period),
            "CUSTOM backup data directory",
            self.daemon
                .debug
                .backup_data_directory
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            "UNSAFE skip backups",
            self.daemon.debug.unsafe_skip_backup,
        )
    }
}

impl Config {
    pub fn default_id<S: Display>(daemon_name: S) -> String {
        format!("{daemon_name}-default")
    }

    pub fn new<S: Into<String>>(daemon_name: S, daemon_home: PathBuf) -> Self {
        let daemon_name = daemon_name.into();

        Config {
            save_path: None,
            nymvisor: Nymvisor {
                id: Self::default_id(&daemon_name),
                debug: Default::default(),
            },
            daemon: Daemon {
                name: daemon_name,
                home: daemon_home,
                debug: Default::default(),
            },
        }
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

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.nymvisor.id)
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        save_formatted_config_to_file(self, path)
    }

    // this code will be needed for config upgrades
    #[allow(dead_code)]
    pub fn try_save(&self) -> io::Result<()> {
        if let Some(save_location) = &self.save_path {
            save_formatted_config_to_file(self, save_location)
        } else {
            warn!("config file save location is unknown. falling back to the default");
            self.save_to_default_location()
        }
    }

    // e.g. $HOME/.nym/nym-api/<id>/nymvisor
    pub fn daemon_nymvisor_dir(&self) -> PathBuf {
        self.daemon.home.join(NYMVISOR_DIR)
    }

    // e.g. $HOME/.nym/nym-apis/<id>/nymvisor/backups
    pub fn daemon_backup_dir(&self) -> PathBuf {
        if let Some(backup_dir) = &self.daemon.debug.backup_data_directory {
            backup_dir.clone()
        } else {
            self.daemon_nymvisor_dir().join(BACKUP_DIR)
        }
    }

    // e.g. $HOME/.nym/nym-apis/<id>/nymvisor/backups/<upgrade-name>
    pub fn daemon_upgrade_backup_dir<P: AsRef<Path>>(&self, upgrade_name: P) -> PathBuf {
        self.daemon_backup_dir().join(upgrade_name)
    }

    // e.g. $HOME/.nym/nym-api/<id>/nymvisor/current-version.json
    pub fn current_daemon_version_filepath(&self) -> PathBuf {
        self.daemon_nymvisor_dir().join(CURRENT_VERSION_FILENAME)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/
    pub fn upgrade_data_dir(&self) -> PathBuf {
        self.nymvisor
            .debug
            .upgrade_data_directory
            .clone()
            .unwrap_or(default_global_data_directory().join(&self.daemon.name))
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/genesis
    pub fn genesis_daemon_dir(&self) -> PathBuf {
        self.upgrade_data_dir().join(GENESIS_DIR)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/genesis/bin/nym-api
    pub fn genesis_daemon_binary(&self) -> PathBuf {
        self.genesis_daemon_dir()
            .join(BIN_DIR)
            .join(&self.daemon.name)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/current
    pub fn current_daemon_dir(&self) -> PathBuf {
        self.upgrade_data_dir().join(CURRENT_DIR)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/current/bin/nym-api
    pub fn current_daemon_binary(&self) -> PathBuf {
        self.current_daemon_dir()
            .join(BIN_DIR)
            .join(&self.daemon.name)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/current/upgrade-info.json
    pub fn current_upgrade_info_filepath(&self) -> PathBuf {
        self.current_daemon_dir().join(UPGRADE_INFO_FILENAME)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/upgrades/<upgrade-name>/upgrade-info.json
    // or $HOME/.nym/nymvisors/data/nym-api/genesis/upgrade-info.json
    pub fn upgrade_info_filepath<S: AsRef<str>>(&self, upgrade_name: S) -> PathBuf {
        // special case for genesis
        let name = upgrade_name.as_ref();
        if name == GENESIS_DIR {
            self.genesis_daemon_dir().join(UPGRADE_INFO_FILENAME)
        } else {
            self.upgrade_dir(name).join(UPGRADE_INFO_FILENAME)
        }
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/upgrades/<upgrade-name>
    pub fn upgrade_dir<P: AsRef<Path>>(&self, upgrade_name: P) -> PathBuf {
        self.upgrades_dir().join(upgrade_name)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/upgrades/<upgrade-name>/bin
    pub fn upgrade_binary_dir<P: AsRef<Path>>(&self, upgrade_name: P) -> PathBuf {
        self.upgrade_dir(upgrade_name).join(BIN_DIR)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/upgrades/<upgrade-name>/bin/nym-api
    pub fn upgrade_binary<P: AsRef<Path>>(&self, upgrade_name: P) -> PathBuf {
        self.upgrade_binary_dir(upgrade_name)
            .join(&self.daemon.name)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/upgrades/<upgrade-name>/bin/nym-api.tmp
    pub fn temp_upgrade_binary<P: AsRef<Path>>(&self, upgrade_name: P) -> PathBuf {
        self.upgrade_binary_dir(upgrade_name)
            .join(format!("{}.tmp", self.daemon.name))
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/upgrades/
    pub fn upgrades_dir(&self) -> PathBuf {
        self.upgrade_data_dir().join(UPGRADES_DIR)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/upgrade-plan.json
    pub fn upgrade_plan_filepath(&self) -> PathBuf {
        self.upgrade_data_dir().join(UPGRADE_PLAN_FILENAME)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/upgrade-history.json
    pub fn upgrade_history_filepath(&self) -> PathBuf {
        self.upgrade_data_dir().join(UPGRADE_HISTORY_FILENAME)
    }

    // e.g. $HOME/.nym/nymvisors/data/nym-api/upgrade.lock
    pub fn upgrade_lock_filepath(&self) -> PathBuf {
        self.upgrade_data_dir().join(UPGRADE_LOCK_FILENAME)
    }

    pub fn upstream_upgrade_url(&self) -> Url {
        if let Some(absolute_url) = &self.daemon.debug.absolute_upstream_upgrade_url {
            absolute_url.clone()
        } else {
            let mut base = self.nymvisor.debug.upstream_base_upgrade_url.clone();
            let prefix = base.path().trim_end_matches('/');
            let daemon = &self.daemon.name;

            base.set_path(&format!("{prefix}/{daemon}/{UPGRADE_INFO_FILENAME}"));
            base
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Nymvisor {
    /// ID specifies the human readable ID of this particular nymvisor instance.
    /// Can be overridden with $NYMVISOR_ID environmental variable.
    pub id: String,

    /// Further optional configuration options associated with the nymvisor.
    #[serde(flatten)]
    pub debug: NymvisorDebug,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NymvisorDebug {
    /// Sets the base url of the upstream source for obtaining upgrade information for the deaemon.
    /// default: "https://nymtech.net/.wellknown/"
    /// It will be used fo constructing the full url, i.e. $NYMVISOR_UPSTREAM_BASE_UPGRADE_URL/$DAEMON_NAME/upgrade-info.json
    /// Can be overridden with $NYMVISOR_UPSTREAM_BASE_UPGRADE_URL environmental variable.
    pub upstream_base_upgrade_url: Url,

    /// Specifies the rate of polling the upstream url for upgrade information.
    /// default: 1h
    /// Can be overridden with $NYMVISOR_UPSTREAM_POLLING_RATE
    #[serde(with = "humantime_serde")]
    pub upstream_polling_rate: Duration,

    /// If set to true, this will disable `nymvisor` logs (but not the underlying process)
    /// default: false
    /// Can be overridden with $NYMVISOR_DISABLE_LOGS environmental variable.
    pub disable_logs: bool,

    /// Set custom directory for upgrade data - binaries and upgrade plans.
    /// If not set, the global nymvisors' data directory will be used instead.
    /// Can be overridden with $NYMVISOR_UPGRADE_DATA_DIRECTORY environmental variable.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub upgrade_data_directory: Option<PathBuf>,
}

impl Default for NymvisorDebug {
    fn default() -> Self {
        NymvisorDebug {
            // this expect is fine as we're parsing a constant, hardcoded value that should always be valid
            #[allow(clippy::expect_used)]
            upstream_base_upgrade_url: DEFAULT_BASE_UPSTREAM_UPGRADE_INFO_SOURCE
                .parse()
                .expect("default upstream url was malformed"),
            upstream_polling_rate: DEFAULT_UPSTREAM_POLLING_RATE,
            disable_logs: false,
            upgrade_data_directory: None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Daemon {
    /// The name of the managed binary itself (e.g. nym-api, nym-mixnode, nym-gateway, etc.)
    /// Can be overridden with $DAEMON_NAME environmental variable.
    pub name: String,

    /// The location where the `nymvisor/` directory is kept that contains the auxiliary files associated
    /// with the underlying daemon, such as any backups or current version information.
    /// (e.g. $HOME/.nym/nym-api/my-nym-api, $HOME/.nym/mixnodes/my-mixnode, etc.).
    /// Can be overridden with $DAEMON_HOME environmental variable.
    pub home: PathBuf,

    /// Further optional configuration options associated with the daemon.
    #[serde(flatten)]
    pub debug: DaemonDebug,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DaemonDebug {
    /// Override url to the upstream source for upgrade plans for this daeamon.
    /// The Url has to point to an endpoint containing a valid [`UpgradeInfo`] json.
    /// Note: if set this takes precedence over .nymvisor.debug.upstream_base_upgrade_url
    /// default: None
    /// Can be overridden with $DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL environmental variable.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub absolute_upstream_upgrade_url: Option<Url>,

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
    /// no further restarts will be attempted and `nymvisor` will exit.
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
    /// Can be overridden with $DAEMON_BACKUP_DATA_DIRECTORY environmental variable.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub backup_data_directory: Option<PathBuf>,

    /// If enabled, `nymvisor` will perform upgrades directly without performing any backups.
    /// default: false
    /// Can be overridden with $DAEMON_UNSAFE_SKIP_BACKUP environmental variable.
    pub unsafe_skip_backup: bool,
}

impl Default for DaemonDebug {
    fn default() -> Self {
        DaemonDebug {
            absolute_upstream_upgrade_url: None,
            allow_binaries_download: true,
            enforce_download_checksum: true,
            restart_after_upgrade: true,
            restart_on_failure: false,
            failure_restart_delay: DEFAULT_FAILURE_RESTART_DELAY,
            max_startup_failures: DEFAULT_MAX_STARTUP_FAILURES,
            startup_period_duration: DEFAULT_STARTUP_PERIOD,
            shutdown_grace_period: DEFAULT_SHUTDOWN_GRACE_PERIOD,
            backup_data_directory: None,
            unsafe_skip_backup: false,
        }
    }
}
