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
use config::NymConfig;
use std::fmt::Display;
use std::process;
use version_checker::{parse_version, Version};

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

fn pre_090_upgrade(from: &str, mut config: Config) -> Config {
    // this is not extracted to separate function as you only have to manually pass version
    // if upgrading from pre090 version
    let from = match from.strip_prefix("v") {
        Some(stripped) => stripped,
        None => from,
    };

    let from = match from.strip_prefix("V") {
        Some(stripped) => stripped,
        None => from,
    };

    let from_version = parse_version(from).expect("invalid version provided!");
    if from_version.major == 0 && from_version.minor < 8 {
        // technically this could be implemented, but is there any point in that?
        eprintln!("upgrading node from before v0.8.0 is not supported. Please run `init` with new binary instead");
        process::exit(1)
    }

    // basically if we're only 0.9.0-dev or 0.9.0-rc1 or whatever, we should preserve that suffix,
    // however, if this is an intermediate upgrade step, set it temporarily to 0.9.0
    let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let to_version = if current.major == 0 && current.minor == 9 {
        current
    } else {
        Version::new(0, 9, 0)
    };

    print_start_upgrade(&from_version, &to_version);

    config
        .get_base_mut()
        .set_custom_version(to_version.to_string().as_ref());

    config.save_to_file(None).unwrap_or_else(|err| {
        eprintln!("failed to overwrite config file! - {:?}", err);
        print_failed_upgrade(&from_version, &to_version);
        process::exit(1);
    });

    print_successful_upgrade(from_version, to_version);

    config
}

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("upgrade").about("Try to upgrade the mixnode")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-client we want to upgrade")
                .takes_value(true)
                .required(true),
        )
        // the rest of arguments depend on the upgrade path
        .arg(Arg::with_name("current version")
            .long("current-version")
            .help("REQUIRED FOR PRE-0.9.0 UPGRADES. Self provided version of the nym-client if none is available in the config. NOTE: if provided incorrectly, results may be catastrophic.")
            .takes_value(true)
        )
}

pub fn execute(matches: &ArgMatches) {
    let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

    let id = matches.value_of("id").unwrap();

    let mut existing_config = Config::load_from_file(None, Some(id)).unwrap_or_else(|err| {
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

    let config_version =
        Version::parse(existing_config.get_base().get_version()).unwrap_or_else(|err| {
            eprintln!("failed to parse node version! - {:?}", err);
            process::exit(1)
        });

    // here be upgrade path to 0.10.0 and beyond based on version number from config
    if config_version == current {
        println!("You're using the most recent version!");
    } else {
        eprintln!("Cannot perform upgrade from {} to {}. Please let the developers know about this issue!", config_version, current)
    }
}
