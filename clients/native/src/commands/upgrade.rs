// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use crate::commands::try_load_current_config;
use clap::Args;
use nym_bin_common::version_checker::Version;
use std::process;

fn unimplemented_upgrade(current_version: &Version, config_version: &Version) -> ! {
    eprintln!("Cannot perform upgrade from {config_version} to {current_version} as it hasn't been implemented yet");
    process::exit(1)
}

#[derive(Args, Clone)]
pub(crate) struct Upgrade {
    /// Id of the nym-client we want to upgrade
    #[clap(long)]
    id: String,
}

fn parse_config_version(config: &Config) -> Version {
    let version = Version::parse(&config.base.client.version).unwrap_or_else(|err| {
        eprintln!("failed to parse client version! - {err}");
        process::exit(1)
    });

    if version.is_prerelease() || !version.build.is_empty() {
        eprintln!(
            "Trying to upgrade from a non-released version {version}. This is not supported!"
        );
        process::exit(1)
    }

    version
}

fn parse_package_version() -> Version {
    let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

    // technically this is not a correct way of checking it as a released version might contain valid build identifiers
    // however, we are not using them ourselves at the moment and hence it should be fine.
    // if we change our mind, we could easily tweak this code
    if version.is_prerelease() || !version.build.is_empty() {
        eprintln!("Trying to upgrade to a non-released version {version}. This is not supported!");
        process::exit(1)
    }

    version
}

fn do_upgrade(config: Config, _args: &Upgrade, package_version: &Version) {
    let config_version = parse_config_version(&config);
    if &config_version == package_version {
        println!("You're using the most recent version!");
        return;
    }

    unimplemented_upgrade(package_version, &config_version)
}

pub(crate) fn execute(args: &Upgrade) {
    let package_version = parse_package_version();

    let id = &args.id;

    let existing_config = try_load_current_config(id).unwrap_or_else(|err| {
        eprintln!("failed to load existing config file! - {err}");
        process::exit(1)
    });

    if existing_config.base.client.version.is_empty() {
        eprintln!("the existing configuration file does not seem to contain version number.");
        process::exit(1);
    }

    do_upgrade(existing_config, args, &package_version)
}
