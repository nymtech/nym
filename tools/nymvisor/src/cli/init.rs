// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{default_config_filepath, Config, BIN_DIR};
use crate::daemon::helpers::get_daemon_build_information;
use crate::env::Env;
use crate::error::NymvisorError;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_bin_common::logging::{setup_logging, setup_tracing_logger};
use nym_bin_common::output_format::OutputFormat;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, error, info, trace, warn};

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Path to the daemon's executable.
    daemon_binary: PathBuf,

    /// ID specifies the human readable ID of this particular nymvisor instance.
    /// Can be overridden with $NYMVISOR_ID environmental variable.
    #[arg(long)]
    id: Option<String>,

    /// If enabled, this will disable `nymvisor` logs (but not the underlying process)
    /// Can be overridden with $NYMVISOR_DISABLE_LOGS environmental variable.
    #[arg(long)]
    disable_nymvisor_logs: bool,

    /// Set custom directory for upgrade data - binaries and upgrade plans.
    /// If not set, the global nymvisors' data directory will be used instead.
    /// Can be overridden with $NYMVISOR_UPGRADE_DATA_DIRECTORY environmental variable.
    #[arg(long)]
    upgrade_data_directory: Option<PathBuf>,

    /// The location where the `nymvisor/` directory is kept that contains the auxiliary files associated
    /// with the underlying daemon, such as any backups or current version information.
    /// (e.g. $HOME/.nym/nym-api/my-nym-api, $HOME/.nym/mixnodes/my-mixnode, etc.).
    /// Can be overridden with $DAEMON_HOME environmental variable.
    #[arg(long)]
    daemon_home: Option<PathBuf>,

    /// If set to true, this will enable auto-downloading of new binaries using the url provided in the `upgrade-info.json`
    /// Can be overridden with $DAEMON_ALLOW_BINARIES_DOWNLOAD environmental variable.
    #[arg(long)]
    allow_download_upgrade_binaries: Option<bool>,

    /// If enabled nymvisor will require that a checksum is provided in the upgrade plan for the binary to be downloaded.
    /// If disabled, nymvisor will not require a checksum to be provided, but still check the checksum if one is provided.
    /// Can be overridden with $DAEMON_ENFORCE_DOWNLOAD_CHECKSUM environmental variable.
    #[arg(long)]
    enforce_download_checksum: Option<bool>,

    /// If enabled, nymvisor will restart the subprocess with the same command-line arguments and flags (but with the new binary) after a successful upgrade.
    /// Otherwise (if disabled), nymvisor will stop running after an upgrade and will require the system administrator to manually restart it.
    /// Note restart is only after the upgrade and does not auto-restart the subprocess after an error occurs.
    /// Can be overridden with $DAEMON_RESTART_AFTER_UPGRADE environmental variable.
    #[arg(long)]
    restart_daemon_after_upgrade: Option<bool>,

    /// If enabled, nymvisor will restart the subprocess with the same command-line arguments and flags after it has crashed
    /// Can be overridden with $DAEMON_RESTART_ON_FAILURE environmental variable.
    #[arg(long)]
    restart_daemon_on_failure: bool,

    /// If `restart_on_failure` is enabled, the following value defines the amount of time `nymvisor` shall wait before
    /// restarting the subprocess.
    /// Can be overridden with $DAEMON_FAILURE_RESTART_DELAY environmental variable.
    #[arg(long, value_parser = humantime::parse_duration)]
    on_failure_daemon_restart_delay: Option<Duration>,

    /// Defines the maximum number of startup failures the subprocess can experience in a quick succession before
    /// no further restarts will be attempted and `nymvisor` will exit.
    /// Can be overridden with $DAEMON_MAX_STARTUP_FAILURES environmental variable.
    #[arg(long)]
    max_daemon_startup_failures: Option<usize>,

    /// Defines the length of time during which the subprocess is still considered to be in the startup phase
    /// when its failures are going to be considered in `max_startup_failures`.
    /// Can be overridden with $DAEMON_STARTUP_PERIOD_DURATION environmental variable.
    #[arg(long, value_parser = humantime::parse_duration)]
    startup_period_duration: Option<Duration>,

    /// Specifies the amount of time `nymvisor` is willing to wait for the subprocess to undergo graceful shutdown after receiving an interrupt
    /// (for either an upgrade or shutdown of the `nymvisor` itself)
    /// Once the time passes, a kill signal is going to be sent instead.
    /// Can be overridden with $DAEMON_SHUTDOWN_GRACE_PERIOD environmental variable.
    #[arg(long, value_parser = humantime::parse_duration)]
    daemon_shutdown_grace_period: Option<Duration>,

    /// Set custom backup directory for daemon data. If not set, the daemon's home directory will be used instead.
    /// Can be overridden with $DAEMON_BACKUP_DATA_DIRECTORY environmental variable.
    daemon_backup_data_directory: Option<PathBuf>,

    /// If enabled, `nymvisor` will perform upgrades directly without performing any backups.
    /// default: false
    /// Can be overridden with $DAEMON_UNSAFE_SKIP_BACKUP environmental variable.
    #[arg(long)]
    unsafe_skip_backup: bool,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl Args {
    pub(crate) fn override_config(&self, config: &mut Config) {
        if let Some(nymvisor_id) = &self.id {
            config.nymvisor.id = nymvisor_id.clone();
        }
        if self.disable_nymvisor_logs {
            config.nymvisor.debug.disable_logs = self.disable_nymvisor_logs;
        }
        if let Some(nymvisor_upgrade_data_directory) = &self.upgrade_data_directory {
            config.nymvisor.debug.upgrade_data_directory =
                Some(nymvisor_upgrade_data_directory.clone());
        }
        if let Some(daemon_home) = &self.daemon_home {
            config.daemon.home = daemon_home.clone();
        }
        if let Some(daemon_allow_binaries_download) = self.allow_download_upgrade_binaries {
            config.daemon.debug.allow_binaries_download = daemon_allow_binaries_download;
        }
        if let Some(enforce_download_checksum) = self.enforce_download_checksum {
            config.daemon.debug.enforce_download_checksum = enforce_download_checksum;
        }
        if let Some(restart_daemon_after_upgrade) = self.restart_daemon_after_upgrade {
            config.daemon.debug.restart_after_upgrade = restart_daemon_after_upgrade;
        }
        if self.restart_daemon_on_failure {
            config.daemon.debug.restart_on_failure = self.restart_daemon_on_failure;
        }
        if let Some(on_failure_daemon_restart_delay) = self.on_failure_daemon_restart_delay {
            config.daemon.debug.failure_restart_delay = on_failure_daemon_restart_delay;
        }
        if let Some(max_daemon_startup_failures) = self.max_daemon_startup_failures {
            config.daemon.debug.max_startup_failures = max_daemon_startup_failures;
        }
        if let Some(startup_period_duration) = self.startup_period_duration {
            config.daemon.debug.startup_period_duration = startup_period_duration;
        }
        if let Some(daemon_shutdown_grace_period) = self.daemon_shutdown_grace_period {
            config.daemon.debug.shutdown_grace_period = daemon_shutdown_grace_period;
        }
        if let Some(daemon_backup_data_directory) = &self.daemon_backup_data_directory {
            config.daemon.debug.backup_data_directory = Some(daemon_backup_data_directory.clone());
        }
        if self.unsafe_skip_backup {
            config.daemon.debug.unsafe_skip_backup = self.unsafe_skip_backup;
        }
    }
}

fn try_build_config(
    args: &Args,
    env: &Env,
    daemon_info: &BinaryBuildInformationOwned,
) -> Result<Config, NymvisorError> {
    let daemon_name = &daemon_info.binary_name;
    let daemon_home = daemon_home(args, env)?;

    debug!(
        "building config for '{daemon_name}' with home at {}",
        daemon_home.display()
    );

    let mut config = Config::new(daemon_name, daemon_home);

    // override config with environmental variables
    debug!("overriding the config with command line arguments");
    args.override_config(&mut config);

    // and then override the result with the passed arguments
    debug!("overriding the config with environmental variables");
    env.override_config(&mut config);

    Ok(config)
}

fn daemon_home(args: &Args, env: &Env) -> Result<PathBuf, NymvisorError> {
    if let Some(home) = &args.daemon_home {
        Ok(home.clone())
    } else if let Some(home) = &env.daemon_home {
        Ok(home.clone())
    } else {
        Err(NymvisorError::DaemonHomeUnavailable)
    }
}

fn use_logs(args: &Args, env: &Env) -> bool {
    if args.disable_nymvisor_logs {
        false
    } else if let Some(disable_logs) = env.nymvisor_disable_logs {
        !disable_logs
    } else {
        true
    }
}

fn init_paths(config: &Config) -> Result<(), NymvisorError> {
    fn init_path<P: AsRef<Path>>(path: P) -> Result<(), NymvisorError> {
        let path = path.as_ref();
        trace!("initialising {}", path.display());

        fs::create_dir_all(path).map_err(|source| NymvisorError::PathInitFailure {
            path: path.to_path_buf(),
            source,
        })
    }

    info!("initialising the directory structure");

    init_path(config.daemon_nymvisor_dir())?;
    init_path(config.daemon_backup_dir())?;
    init_path(config.upgrade_data_dir())?;
    init_path(config.genesis_daemon_dir().join(BIN_DIR))?;
    init_path(config.upgrades_dir())?;

    Ok(())
}

fn copy_genesis_binary(
    config: &Config,
    source_dir: &Path,
    daemon_info: &BinaryBuildInformationOwned,
) -> Result<(), NymvisorError> {
    info!("setting up the genesis binary");
    let target = config.genesis_daemon_binary();

    if target.exists() {
        // if there already exists a binary at the genesis location, see if it's the same one
        let existing_bin_info = get_daemon_build_information(&target)?;
        return if &existing_bin_info != daemon_info {
            Err(NymvisorError::DuplicateDaemonGenesisBinary {
                daemon_name: config.daemon.name.clone(),
                existing_info: Box::new(existing_bin_info),
                provided_genesis: Box::new(daemon_info.clone()),
            })
        } else {
            debug!("there was already a genesis daemon binary present, but it was the same as the one provided");
            Ok(())
        };
    }

    // TODO: setup initial upgrade-info.json file
    fs::copy(source_dir, &target).map_err(|source| NymvisorError::DaemonBinaryCopyFailure {
        source_path: source_dir.to_path_buf(),
        target_path: target,
        source,
    })?;
    Ok(())
}

fn create_current_symlink(config: &Config) -> Result<(), NymvisorError> {
    info!("setting up the symlink to the genesis directory");

    let original = config.genesis_daemon_dir();
    let link = config.current_daemon_dir();

    // check if a symlink already exists
    if let Ok(existing_target) = fs::read_link(&link) {
        return if existing_target != original {
            Err(NymvisorError::ExistingCurrentSymlink {
                daemon_name: config.daemon.name.clone(),
                link: existing_target,
                expected_link: original,
            })
        } else {
            debug!(
                "there already exist a symlink between {} and {}",
                original.display(),
                link.display()
            );
            Ok(())
        };
    }

    std::os::unix::fs::symlink(&original, &link).map_err(|source| {
        NymvisorError::SymlinkCreationFailure {
            source_path: original,
            target_path: link,
            source,
        }
    })
}

fn save_config(config: Config, env: &Env) -> Result<(), NymvisorError> {
    let id = &config.nymvisor.id;
    let config_save_location = env
        .nymvisor_config_path
        .clone()
        .unwrap_or(default_config_filepath(id));

    info!(
        "saving the config file to {}",
        config_save_location.display()
    );

    config
        .save_to_path(&config_save_location)
        .map_err(|err| NymvisorError::ConfigSaveFailure {
            path: config_save_location,
            id: id.to_string(),
            source: err,
        })?;
    Ok(())
}

/// Initialise the nymvisor by performing the following:
/// - [✅] executing the `build-info` command on the daemon executable to check its validity and obtain its name
/// - [✅] creating `<DAEMON_HOME>/nymvisor` folder if it doesn't yet exist
/// - [✅] creating `<DAEMON_BACKUP_DATA_DIRECTORY>` folder if it doesn't yet exist
/// - [✅] creating `<NYMVISOR_UPGRADE_DATA_DIRECTORY>` folder if it doesn't yet exist
/// - [✅] creating `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/genesis/bin` folder if it doesn't yet exist
/// - [✅] creating `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/upgrades` folder if it doesn't yet exist
/// - [⚠️] copying the provided executable to `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/genesis/bin/<DAEMON_NAME>`
/// - [✅] creating a `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/current` symlink pointing to `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/genesis`
/// - [✅] saving nymvisor's config file to `<NYMVISOR_CONFIG_PATH>` and creating the full directory structure.
///
/// note: it requires either passing `--daemon-home` flag or setting the `$DAEMON_HOME` environmental variable
pub(crate) fn execute(args: Args) -> Result<(), NymvisorError> {
    let env = Env::try_read()?;

    if use_logs(&args, &env) {
        setup_tracing_logger();
        info!("enabled nymvisor logging");
    }

    info!("initialising the nymvisor");

    // this serves two purposes:
    // 1. we get daemon name if it wasn't provided via either a flag or env variable
    // 2. we check if valid executable was provided
    let daemon_info = get_daemon_build_information(&args.daemon_binary)?;

    let config = try_build_config(&args, &env, &daemon_info)?;

    init_paths(&config)?;
    copy_genesis_binary(&config, &args.daemon_binary, &daemon_info)?;
    create_current_symlink(&config)?;
    save_config(config, &env)?;

    Ok(())
}
