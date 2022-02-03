// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{missing_string_value, Config};
use clap::Args;
use config::defaults::default_api_endpoints;
use config::NymConfig;
use std::fmt::Display;
use std::process;
use version_checker::Version;

#[derive(Args)]
pub(crate) struct Upgrade {
    /// Id of the nym-mixnode we want to upgrade
    #[clap(long)]
    id: String,
}

#[allow(dead_code)]
fn fail_upgrade<D1: Display, D2: Display>(from_version: D1, to_version: D2) -> ! {
    print_failed_upgrade(from_version, to_version);
    process::exit(1)
}

fn print_start_upgrade<D1: Display, D2: Display>(from: D1, to: D2) {
    println!(
        "\n==================\nTrying to upgrade mixnode from {} to {} ...",
        from, to
    );
}

fn print_failed_upgrade<D1: Display, D2: Display>(from: D1, to: D2) {
    eprintln!(
        "Upgrade from {} to {} failed!\n==================\n",
        from, to
    );
}

fn print_successful_upgrade<D1: Display, D2: Display>(from: D1, to: D2) {
    println!(
        "Upgrade from {} to {} was successful!\n==================\n",
        from, to
    );
}

fn outdated_upgrade(config_version: &Version, package_version: &Version) -> ! {
    eprintln!(
        "Cannot perform upgrade from {} to {}. Your version is too old to perform the upgrade.!",
        config_version, package_version
    );
    process::exit(1)
}

fn unsupported_upgrade(config_version: &Version, package_version: &Version) -> ! {
    eprintln!("Cannot perform upgrade from {} to {}. Please let the developers know about this issue if you expected it to work!", config_version, package_version);
    process::exit(1)
}

fn parse_config_version(config: &Config) -> Version {
    let version = Version::parse(config.get_version()).unwrap_or_else(|err| {
        eprintln!("failed to parse client version! - {:?}", err);
        process::exit(1)
    });

    if version.is_prerelease() || !version.build.is_empty() {
        eprintln!(
            "Trying to upgrade from a non-released version {}. This is not supported!",
            version
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
        eprintln!(
            "Trying to upgrade to a non-released version {}. This is not supported!",
            version
        );
        process::exit(1)
    }

    version
}

fn minor_0_12_upgrade(
    config: Config,
    _args: &Upgrade,
    config_version: &Version,
    package_version: &Version,
) -> Config {
    let to_version = if package_version.major == 0 && package_version.minor == 12 {
        package_version.clone()
    } else {
        Version::new(0, 12, 0)
    };

    print_start_upgrade(&config_version, &to_version);

    println!(
        "Setting validator API endpoints to {:?}",
        default_api_endpoints()
    );

    let upgraded_config = config
        .with_custom_version(to_version.to_string().as_ref())
        .with_custom_validator_apis(default_api_endpoints());

    upgraded_config.save_to_file(None).unwrap_or_else(|err| {
        eprintln!("failed to overwrite config file! - {:?}", err);
        print_failed_upgrade(&config_version, &to_version);
        process::exit(1);
    });

    print_successful_upgrade(config_version, to_version);

    upgraded_config
}

fn do_upgrade(mut config: Config, args: &Upgrade, package_version: Version) {
    loop {
        let config_version = parse_config_version(&config);

        if config_version == package_version {
            println!("You're using the most recent version!");
            return;
        }

        config = match config_version.major {
            0 => match config_version.minor {
                9 | 10 => outdated_upgrade(&config_version, &package_version),
                11 => minor_0_12_upgrade(config, args, &config_version, &package_version),
                _ => unsupported_upgrade(&config_version, &package_version),
            },
            _ => unsupported_upgrade(&config_version, &package_version),
        }
    }
}

pub(crate) fn execute(args: &Upgrade) {
    let package_version = parse_package_version();

    let existing_config = Config::load_from_file(Some(&args.id)).unwrap_or_else(|err| {
        eprintln!("failed to load existing config file! - {:?}", err);
        process::exit(1)
    });

    if existing_config.get_version() == missing_string_value::<String>() {
        eprintln!("the existing configuration file does not seem to contain version number.");
        process::exit(1);
    }

    do_upgrade(existing_config, args, package_version)
}
