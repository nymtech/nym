// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::client::config::{Config, MISSING_VALUE};
use clap::{App, Arg, ArgMatches};
use client_core::config::{default_validator_rest_endpoints, DEFAULT_MIXNET_CONTRACT_ADDRESS};
use config::NymConfig;
use std::fmt::Display;
use std::process;
use version_checker::{parse_version, Version};

fn print_start_upgrade<D1: Display, D2: Display>(from: D1, to: D2) {
    println!(
        "\n==================\nTrying to upgrade client from {} to {} ...",
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

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("upgrade").about("Try to upgrade the client")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-socks5-client we want to upgrade")
                .takes_value(true)
                .required(true),
        )
        // the rest of arguments depend on the upgrade path
        .arg(Arg::with_name("current version")
            .long("current-version")
            .help("REQUIRED FOR PRE-0.9.0 UPGRADES. Specifies current version of the configuration file to help to determine a valid upgrade path. Valid formats include '0.8.1', 'v0.8.1' or 'V0.8.1'")
            .takes_value(true)
        )
}

fn unsupported_upgrade(config_version: Version, package_version: Version) -> ! {
    eprintln!("Cannot perform upgrade from {} to {}. Please let the developers know about this issue if you expected it to work!", config_version, package_version);
    process::exit(1)
}

fn parse_config_version(config: &Config) -> Version {
    let version = Version::parse(config.get_base().get_version()).unwrap_or_else(|err| {
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

fn pre_090_upgrade(from: &str, mut config: Config) -> Config {
    // this is not extracted to separate function as you only have to manually pass version
    // if upgrading from pre090 version
    let from = match from.strip_prefix('v') {
        Some(stripped) => stripped,
        None => from,
    };

    let from = match from.strip_prefix('V') {
        Some(stripped) => stripped,
        None => from,
    };

    let from_version = parse_version(from).expect("invalid version provided!");
    if from_version.major == 0 && from_version.minor < 8 {
        // technically this could be implemented, but is there any point in that?
        eprintln!("upgrading client from before v0.8.0 is not supported. Please run `init` with new binary instead");
        process::exit(1)
    }

    if (from_version.major == 0 && from_version.minor >= 9) || from_version.major >= 1 {
        eprintln!("self reported version is higher than 0.9.0. Those releases should have already contained version numbers in config! Make sure you provided correct version");
        process::exit(1)
    }

    // note: current is guaranteed to not have any `build` information suffix (nor pre-release
    // information), as this was asserted at the beginning of this command)
    //
    // upgrade to current (if it's a 0.9.X) or try to upgrade to 0.9.0 as an intermediate
    // step in future upgrades (so, for example, we might go 0.8.0 -> 0.9.0 -> 0.10.0)
    // this way we don't need to have all the crazy paths on how to upgrade from any version to any
    // other version. We just upgrade one minor version at a time.
    let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let to_version = if current.major == 0 && current.minor == 9 {
        current
    } else {
        Version::new(0, 9, 0)
    };

    if config.get_base().get_validator_rest_endpoints()[0] != MISSING_VALUE {
        eprintln!("existing config seems to have specified new validator rest endpoint which was only introduced in 0.9.0! Can't perform upgrade.");
        print_failed_upgrade(&from_version, &to_version);
        process::exit(1);
    }

    print_start_upgrade(&from_version, &to_version);

    config
        .get_base_mut()
        .set_custom_version(to_version.to_string().as_ref());

    println!(
        "Setting validator REST endpoint to {:?}",
        default_validator_rest_endpoints()
    );

    config
        .get_base_mut()
        .set_custom_validators(default_validator_rest_endpoints());

    config.save_to_file(None).unwrap_or_else(|err| {
        eprintln!("failed to overwrite config file! - {:?}", err);
        print_failed_upgrade(&from_version, &to_version);
        process::exit(1);
    });

    print_successful_upgrade(from_version, to_version);

    config
}

/*
changes:
- introduction of mixnet contract address field
- change to default validator rest endpoint
 */
fn minor_010_upgrade(
    mut config: Config,
    _matches: &ArgMatches,
    config_version: &Version,
    package_version: &Version,
) -> Config {
    let to_version = if package_version.major == 0 && package_version.minor == 10 {
        package_version.clone()
    } else {
        Version::new(0, 10, 0)
    };

    print_start_upgrade(&config_version, &to_version);

    config
        .get_base_mut()
        .set_custom_version(to_version.to_string().as_ref());

    if config.get_base().get_validator_mixnet_contract_address() != MISSING_VALUE {
        eprintln!("existing config seems to have specified mixnet contract address which was only introduced in 0.10.0! Can't perform upgrade.");
        print_failed_upgrade(&config_version, &to_version);
        process::exit(1);
    }

    println!(
        "Setting mixnet contract address to {}",
        DEFAULT_MIXNET_CONTRACT_ADDRESS
    );

    config
        .get_base_mut()
        .set_mixnet_contract(DEFAULT_MIXNET_CONTRACT_ADDRESS);

    // The default validator endpoint changed
    println!(
        "Setting validator REST endpoint to to {:?}",
        default_validator_rest_endpoints()
    );

    config
        .get_base_mut()
        .set_custom_validators(default_validator_rest_endpoints());

    config.save_to_file(None).unwrap_or_else(|err| {
        eprintln!("failed to overwrite config file! - {:?}", err);
        print_failed_upgrade(&config_version, &to_version);
        process::exit(1);
    });

    print_successful_upgrade(config_version, to_version);

    config
}

// no changes but version number
fn patch_010_upgrade(
    mut config: Config,
    _matches: &ArgMatches,
    config_version: &Version,
    package_version: &Version,
) -> Config {
    let to_version = package_version;

    print_start_upgrade(&config_version, &to_version);

    config
        .get_base_mut()
        .set_custom_version(to_version.to_string().as_ref());

    config.save_to_file(None).unwrap_or_else(|err| {
        eprintln!("failed to overwrite config file! - {:?}", err);
        print_failed_upgrade(&config_version, &to_version);
        process::exit(1);
    });

    print_successful_upgrade(config_version, to_version);

    config
}

fn do_upgrade(mut config: Config, matches: &ArgMatches, package_version: Version) {
    loop {
        let config_version = parse_config_version(&config);

        if config_version == package_version {
            println!("You're using the most recent version!");
            return;
        }

        config = match config_version.major {
            0 => match config_version.minor {
                9 => minor_010_upgrade(config, &matches, &config_version, &package_version),
                10 => patch_010_upgrade(config, &matches, &config_version, &package_version),
                _ => unsupported_upgrade(config_version, package_version),
            },
            _ => unsupported_upgrade(config_version, package_version),
        }
    }
}

pub fn execute(matches: &ArgMatches) {
    let package_version = parse_package_version();

    let id = matches.value_of("id").unwrap();

    let mut existing_config = Config::load_from_file(id).unwrap_or_else(|err| {
        eprintln!("failed to load existing config file! - {:?}", err);
        process::exit(1)
    });

    // versions fields were added in 0.9.0
    if existing_config.get_base().get_version() == MISSING_VALUE {
        let self_reported_version = matches.value_of("current version").unwrap_or_else(|| {
            eprintln!(
                "trying to upgrade from pre v0.9.0 without providing current system version!"
            );
            process::exit(1)
        });

        // upgrades up to 0.9.0
        existing_config = pre_090_upgrade(self_reported_version, existing_config);
    }

    // here be upgrade path to 0.9.X and beyond based on version number from config
    do_upgrade(existing_config, matches, package_version)
}
