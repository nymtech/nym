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

use crate::config::{
    missing_string_value, Config, DEFAULT_METRICS_SERVER, DEFAULT_VALIDATOR_REST_ENDPOINT,
};
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::identity;
use std::fmt::Display;
use std::path::PathBuf;
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

fn pre_090_upgrade(from: &str, config: Config) -> Config {
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

    print_start_upgrade(&from, &to_version);

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
        print_failed_upgrade(&from_version, &to_version);
        process::exit(1)
    }

    if (from_version.major == 0 && from_version.minor >= 9) || from_version.major >= 1 {
        eprintln!("self reported version is higher than 0.9.0. Those releases should have already contained version numbers in config! Make sure you provided correct version");
        print_failed_upgrade(&from_version, &to_version);
        process::exit(1)
    }

    if config.get_private_identity_key_file() != missing_string_value::<PathBuf>()
        || config.get_public_identity_key_file() != missing_string_value::<PathBuf>()
    {
        eprintln!("existing config seems to have specified identity keys which were only introduced in 0.9.0! Can't perform upgrade.");
        print_failed_upgrade(&from_version, &to_version);
        process::exit(1);
    }

    if config.get_validator_rest_endpoint() != missing_string_value::<String>()
        || config.get_metrics_server() != missing_string_value::<String>()
    {
        eprintln!("existing config seems to have specified new validator and/or metrics-server endpoinnts which were only introduced in 0.9.0! Can't perform upgrade.");
        print_failed_upgrade(&from_version, &to_version);
        process::exit(1);
    }

    let mut upgraded_config = config
        .with_custom_version(to_version.to_string().as_ref())
        .with_custom_validator(DEFAULT_VALIDATOR_REST_ENDPOINT)
        .with_custom_metrics_server(DEFAULT_METRICS_SERVER);

    println!(
        "Setting validator REST endpoint to {}",
        DEFAULT_VALIDATOR_REST_ENDPOINT
    );
    println!("Setting metrics server to {}", DEFAULT_METRICS_SERVER);

    println!("Generating new identity...");
    let identity_keys = identity::KeyPair::new();
    upgraded_config.set_default_identity_keypair_paths();

    if let Err(err) = pemstore::store_keypair(
        &identity_keys,
        &pemstore::KeyPairPath::new(
            upgraded_config.get_private_identity_key_file(),
            upgraded_config.get_public_identity_key_file(),
        ),
    ) {
        eprintln!("Failed to save new identity key files! - {}", err);
        process::exit(1);
    }

    upgraded_config.save_to_file(None).unwrap_or_else(|err| {
        eprintln!("failed to overwrite config file! - {:?}", err);
        print_failed_upgrade(&from_version, &to_version);
        process::exit(1);
    });

    print_successful_upgrade(from_version, to_version);

    upgraded_config
}

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("upgrade").about("Try to upgrade the mixnode")
        .arg(
        Arg::with_name("id")
            .long("id")
            .help("Id of the nym-mixnode we want to upgrade")
            .takes_value(true)
            .required(true),
        )
    // the rest of arguments depend on the upgrade path
        .arg(Arg::with_name("current version")
            .long("current-version")
            .help("REQUIRED FOR PRE-0.9.0 UPGRADES. Self provided version of the nym-mixnode if none is available in the config. NOTE: if provided incorrectly, results may be catastrophic.")
            .takes_value(true)
        )
}

pub fn execute(matches: &ArgMatches) {
    let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

    // technically this is not a correct way of checking it as a released version might contain valid build identifiers
    // however, we are not using them ourselves at the moment and hence it should be fine.
    // if we change our mind, we could easily tweak this code
    if current.is_prerelease() || !current.build.is_empty() {
        eprintln!(
            "Trying to upgrade to a non-released version {}. This is not supported!",
            current
        );
        process::exit(1)
    }

    let id = matches.value_of("id").unwrap();

    let mut existing_config = Config::load_from_file(None, Some(id)).unwrap_or_else(|err| {
        eprintln!("failed to load existing config file! - {:?}", err);
        process::exit(1)
    });

    // versions fields were added in 0.9.0
    if existing_config.get_version() == missing_string_value::<String>() {
        let self_reported_version = matches.value_of("current version").unwrap_or_else(|| {
            eprintln!(
                "trying to upgrade from pre v0.9.0 without providing current system version!"
            );
            process::exit(1)
        });

        // upgrades up to 0.9.0
        existing_config = pre_090_upgrade(self_reported_version, existing_config);
    }

    let config_version = Version::parse(existing_config.get_version()).unwrap_or_else(|err| {
        eprintln!("failed to parse node version! - {:?}", err);
        process::exit(1)
    });

    if config_version.is_prerelease() || !config_version.build.is_empty() {
        eprintln!(
            "Trying to upgrade to from non-released version {}. This is not supported!",
            current
        );
        process::exit(1)
    }

    // here be upgrade path to 0.10.0 and beyond based on version number from config
    if config_version == current {
        println!("You're using the most recent version!");
    } else {
        eprintln!("Cannot perform upgrade from {} to {}. Please let the developers know about this issue!", config_version, current);
        process::exit(1)
    }
}
