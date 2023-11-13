// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::helpers::copy_binary;
use crate::cli::try_load_current_config;
use crate::daemon::Daemon;
use crate::env::Env;
use crate::error::NymvisorError;
use crate::helpers::init_path;
use crate::upgrades::types::{UpgradeInfo, UpgradePlan};
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const DEFAULT_UPGRADE_DELAY: Duration = Duration::from_secs(15 * 60);

fn parse_rfc3339_upgrade_time(raw: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(raw, &Rfc3339)
}

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Path to the daemon's upgrade executable.
    daemon_binary: PathBuf,

    /// Name of this upgrade
    #[arg(long)]
    upgrade_name: String,

    /// Overwrite existing upgrade binary / upgrade-info.json file
    #[arg(long)]
    force: bool,

    /// Force the upgrade to happen immediately
    #[arg(long, group = "time")]
    now: bool,

    /// Specifies the additional metadata of this upgrade to set the publish date of this upgrade.
    /// If unset, the current time will be used.
    #[arg(long, value_parser = parse_rfc3339_upgrade_time)]
    publish_date: Option<OffsetDateTime>,

    /// Specifies the time at which the provided upgrade will be performed (RFC3339 formatted).
    /// If left unset, the upgrade will be performed in 15min
    #[arg(long, value_parser = parse_rfc3339_upgrade_time, group = "time")]
    upgrade_time: Option<OffsetDateTime>,

    /// Specifies delay until the provided upgrade is going to get performed.
    /// If let unset, the upgrade will be performed in 15min
    #[arg(long, value_parser = humantime::parse_duration, group = "time")]
    upgrade_delay: Option<Duration>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl Args {
    fn determine_upgrade_time(&self) -> OffsetDateTime {
        // 1. there's always going to be at most one of: upgrade-delay, upgrade-time or now
        // 2. if missing use 15min
        if let Some(upgrade_time) = self.upgrade_time {
            upgrade_time
        } else if let Some(upgrade_delay) = self.upgrade_delay {
            OffsetDateTime::now_utc() + upgrade_delay
        } else if self.now {
            OffsetDateTime::now_utc()
        } else {
            OffsetDateTime::now_utc() + DEFAULT_UPGRADE_DELAY
        }
    }
}

pub(crate) fn execute(args: Args) -> Result<(), NymvisorError> {
    let mut env = Env::try_read()?;

    let tmp_daemon = Daemon::new(&args.daemon_binary);
    tmp_daemon.verify_binary()?;

    let bin_info = tmp_daemon.get_build_information()?;
    if env.daemon_name.is_none() {
        env.daemon_name = Some(bin_info.binary_name.clone());
    }

    let config = try_load_current_config(&env)?;

    let mut current_upgrade_plan = UpgradePlan::try_load(config.upgrade_plan_filepath())?;

    let upgrade_time = args.determine_upgrade_time();
    let upgrade_info = UpgradeInfo {
        manual: false,
        name: args.upgrade_name,
        notes: "manually added via 'add-upgrade' command".to_string(),
        publish_date: Some(args.publish_date.unwrap_or(OffsetDateTime::now_utc())),
        version: bin_info.build_version.clone(),
        platforms: Default::default(),
        upgrade_time,
        binary_details: Some(bin_info),
    };

    let path = config.upgrade_dir(&upgrade_info.name);
    if path.exists() && !args.force {
        return Err(NymvisorError::ExistingUpgrade {
            name: upgrade_info.name,
            path,
        });
    }

    init_path(config.upgrade_binary_dir(&upgrade_info.name))?;
    copy_binary(
        &args.daemon_binary,
        config.upgrade_binary(&upgrade_info.name),
    )?;
    upgrade_info.save(config.upgrade_info_filepath(&upgrade_info.name))?;
    current_upgrade_plan.insert_new_upgrade(upgrade_info)?;

    Ok(())
}
