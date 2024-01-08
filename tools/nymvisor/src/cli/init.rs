// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::helpers::{copy_binary, daemon_home, use_logs};
use crate::config::{
    default_config_filepath, Config, BIN_DIR, CURRENT_VERSION_FILENAME, GENESIS_DIR,
};
use crate::daemon::Daemon;
use crate::env::Env;
use crate::error::NymvisorError;
use crate::helpers::init_path;
use crate::upgrades::types::{CurrentVersionInfo, UpgradeInfo, UpgradePlan};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_bin_common::logging::setup_tracing_logger;
use nym_bin_common::output_format::OutputFormat;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{debug, info, warn};
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Path to the daemon's executable.
    daemon_binary: PathBuf,

    /// ID specifies the human readable ID of this particular nymvisor instance.
    /// Can be overridden with $NYMVISOR_ID environmental variable.
    #[arg(long)]
    id: Option<String>,

    /// Sets the base url of the upstream source for obtaining upgrade information for the deaemon.
    /// It will be used fo constructing the full url, i.e. $NYMVISOR_UPSTREAM_BASE_UPGRADE_URL/$DAEMON_NAME/upgrade-info.json
    /// Can be overridden with $NYMVISOR_UPSTREAM_BASE_UPGRADE_URL environmental variable.
    #[arg(long)]
    upstream_base_upgrade_url: Option<Url>,

    /// Specifies the rate of polling the upstream url for upgrade information.
    /// default: 1h
    /// Can be overridden with $NYMVISOR_UPSTREAM_POLLING_RATE
    #[arg(long, value_parser = humantime::parse_duration)]
    upstream_polling_rate: Option<Duration>,

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

    /// Override url to the upstream source for upgrade plans for this daeamon.
    /// The Url has to point to an endpoint containing a valid [`UpgradeInfo`] json.
    /// Note: if set this takes precedence over `upstream_base_upgrade_url`
    /// Can be overridden with $DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL environmental variable.
    #[arg(long)]
    daemon_absolute_upstream_upgrade_url: Option<Url>,

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
    #[arg(long)]
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
        if let Some(upstream) = &self.upstream_base_upgrade_url {
            config.nymvisor.debug.upstream_base_upgrade_url = upstream.clone()
        }
        if let Some(polling_rate) = self.upstream_polling_rate {
            config.nymvisor.debug.upstream_polling_rate = polling_rate
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
        if let Some(upstream) = &self.daemon_absolute_upstream_upgrade_url {
            config.daemon.debug.absolute_upstream_upgrade_url = Some(upstream.clone())
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
    let daemon_home = daemon_home(&args.daemon_home, env)?;

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

fn init_paths(config: &Config) -> Result<(), NymvisorError> {
    info!("initialising the directory structure");

    init_path(config.daemon_nymvisor_dir())?;
    init_path(config.daemon_backup_dir())?;
    init_path(config.upgrade_data_dir())?;
    init_path(config.genesis_daemon_dir().join(BIN_DIR))?;
    init_path(config.upgrades_dir())?;

    Ok(())
}

fn setup_daemon_current_version(
    config: &Config,
    daemon_info: &BinaryBuildInformationOwned,
) -> Result<(), NymvisorError> {
    info!("setting up initial {}", CURRENT_VERSION_FILENAME);
    let path = config.current_daemon_version_filepath();

    let initial = CurrentVersionInfo {
        name: GENESIS_DIR.to_string(),
        version: daemon_info.build_version.clone(),
        upgrade_time: OffsetDateTime::now_utc(),
        binary_details: daemon_info.clone(),
    };

    initial.save(path)
}

fn setup_genesis(
    config: &Config,
    source: &Path,
    daemon_info: &BinaryBuildInformationOwned,
) -> Result<(), NymvisorError> {
    info!("setting up the genesis binary");
    let target = config.genesis_daemon_binary();

    if target.exists() {
        // if there already exists a binary at the genesis location, see if it's the same one
        let existing_bin_info = Daemon::new(target).get_build_information()?;
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

    let genesis_info = generate_and_save_genesis_upgrade_info(config, daemon_info)?;
    setup_initial_upgrade_plan(config, genesis_info)?;
    copy_binary(source, target)
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

fn generate_and_save_genesis_upgrade_info(
    config: &Config,
    genesis_info: &BinaryBuildInformationOwned,
) -> Result<UpgradeInfo, NymvisorError> {
    info!("setting up the genesis upgrade-info.json");

    let info = UpgradeInfo {
        manual: true,
        name: GENESIS_DIR.to_string(),
        notes: "".to_string(),
        publish_date: None,
        version: genesis_info.build_version.clone(),
        platforms: Default::default(),
        upgrade_time: OffsetDateTime::UNIX_EPOCH,
        binary_details: Some(genesis_info.clone()),
    };
    let save_path = config.upgrade_info_filepath(&info.name);

    // if the upgrade info file already exists return an error since there is no associated binary
    if save_path.exists() {
        Err(NymvisorError::UpgradeInfoWithNoBinary {
            name: info.name,
            path: save_path,
        })
    } else {
        info.save(save_path)?;
        Ok(info)
    }
}

fn setup_initial_upgrade_plan(
    config: &Config,
    genesis_info: UpgradeInfo,
) -> Result<(), NymvisorError> {
    info!("setting up initial upgrade-plan.json");

    let plan_path = config.upgrade_plan_filepath();

    if plan_path.exists() {
        warn!("there is already an upgrade-plan.json file present");
        // if the file already exists, try to load it and see if the 'current' matches
        let existing_plan = UpgradePlan::try_load(&plan_path)?;
        if let (Some(current_info), Some(existing_info)) = (
            &genesis_info.binary_details,
            &existing_plan.current().binary_details,
        ) {
            if current_info != existing_info {
                // if possible, compare the actual full details
                return Err(NymvisorError::PreexistingUpgradePlan {
                    path: plan_path,
                    current_name: genesis_info.name,
                    existing_name: existing_plan.current().name.clone(),
                });
            }
        } else if genesis_info.name != existing_plan.current().name {
            // otherwise just check the upgrade name
            return Err(NymvisorError::PreexistingUpgradePlan {
                path: plan_path,
                current_name: genesis_info.name,
                existing_name: existing_plan.current().name.clone(),
            });
        }

        return Ok(());
    }

    UpgradePlan::new(genesis_info).save_new(plan_path)
}

fn save_config(config: &Config, env: &Env) -> Result<(), NymvisorError> {
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
/// - executing the `build-info` command on the daemon executable to check its validity and obtain its name
/// - creating `<DAEMON_HOME>/nymvisor` folder if it doesn't yet exist
/// - creating `<DAEMON_BACKUP_DATA_DIRECTORY>` folder if it doesn't yet exist
/// - creating `<NYMVISOR_UPGRADE_DATA_DIRECTORY>` folder if it doesn't yet exist
/// - creating `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/genesis/bin` folder if it doesn't yet exist
/// - creating `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/upgrades` folder if it doesn't yet exist
/// - copying the provided executable to `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/genesis/bin/<DAEMON_NAME>`
/// - generating initial `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/genesis/upgrade-info.json` file
/// - generating initial `<DAEMON_HOME>/nymvisor/current-version-info.json` file
/// - creating a `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/current` symlink pointing to `<NYMVISOR_UPGRADE_DATA_DIRECTORY>/<DAEMON_NAME>/genesis`
/// - saving nymvisor's config file to `<NYMVISOR_CONFIG_PATH>` and creating the full directory structure.
///
/// note: it requires either passing `--daemon-home` flag or setting the `$DAEMON_HOME` environmental variable
pub(crate) fn execute(args: Args) -> Result<(), NymvisorError> {
    let env = Env::try_read()?;

    if use_logs(args.disable_nymvisor_logs, &env) {
        setup_tracing_logger();
        info!("enabled nymvisor logging");
    }

    info!("initialising the nymvisor");

    // this serves two purposes:
    // 1. we get daemon name if it wasn't provided via either a flag or env variable
    // 2. we check if valid executable was provided
    let daemon_info = Daemon::new(args.daemon_binary.clone()).get_build_information()?;

    let config = try_build_config(&args, &env, &daemon_info)?;

    init_paths(&config)?;
    setup_genesis(&config, &args.daemon_binary, &daemon_info)?;
    setup_daemon_current_version(&config, &daemon_info)?;
    create_current_symlink(&config)?;
    save_config(&config, &env)?;

    println!("{}", args.output.format(&config));
    Ok(())
}
