// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::*;
use crate::config::{
    default_validator_rest_endpoints, missing_string_value, Config,
    DEFAULT_MIXNET_CONTRACT_ADDRESS, MISSING_VALUE,
};
use crate::node::node_description::{NodeDescription, DESCRIPTION_FILE};
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::identity;
use serde::Deserialize;
use std::fmt::Display;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::{fs, process};
use version_checker::{parse_version, Version};

type UpgradeError = (Version, String);
const CURRENT_VERSION_ARG_NAME: &str = "current-version";

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

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("upgrade").about("Try to upgrade the mixnode")
        .arg(
            Arg::with_name(ID_ARG_NAME)
                .long(ID_ARG_NAME)
                .help("Id of the nym-mixnode we want to upgrade")
                .takes_value(true)
                .required(true),
        )
        // the rest of arguments depend on the upgrade path
        .arg(Arg::with_name(CURRENT_VERSION_ARG_NAME)
            .long(CURRENT_VERSION_ARG_NAME)
            .help("REQUIRED FOR PRE-0.9.0 UPGRADES. Specifies current version of the configuration file to help to determine a valid upgrade path. Valid formats include '0.8.1', 'v0.8.1' or 'V0.8.1'")
            .takes_value(true)
        )
}

fn unsupported_upgrade(config_version: Version, package_version: Version) -> ! {
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
        eprintln!("upgrading node from before v0.8.0 is not supported. Please run `init` with new binary instead");
        fail_upgrade(&from_version, &to_version)
    }

    if (from_version.major == 0 && from_version.minor >= 9) || from_version.major >= 1 {
        eprintln!("self reported version is higher than 0.9.0. Those releases should have already contained version numbers in config! Make sure you provided correct version");
        fail_upgrade(&from_version, &to_version)
    }

    if config.get_private_identity_key_file() != missing_string_value::<PathBuf>()
        || config.get_public_identity_key_file() != missing_string_value::<PathBuf>()
    {
        eprintln!("existing config seems to have specified identity keys which were only introduced in 0.9.0! Can't perform upgrade.");
        fail_upgrade(&from_version, &to_version)
    }

    if config.get_validator_rest_endpoints()[0] != missing_string_value::<String>() {
        eprintln!("existing config seems to have specified new validator rest endpoint which was only introduced in 0.9.0! Can't perform upgrade.");
        fail_upgrade(&from_version, &to_version)
    }

    let mut upgraded_config = config
        .with_custom_version(to_version.to_string().as_ref())
        .with_custom_validators(default_validator_rest_endpoints());

    println!(
        "Setting validator REST endpoints to {:?}",
        default_validator_rest_endpoints()
    );

    println!("Generating new identity...");
    let mut rng = rand::rngs::OsRng;

    let identity_keys = identity::KeyPair::new(&mut rng);
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
        fail_upgrade(&from_version, &to_version)
    });

    print_successful_upgrade(from_version, to_version);

    upgraded_config
}

fn minor_0_10_upgrade(
    config: Config,
    _matches: &ArgMatches,
    config_version: &Version,
    package_version: &Version,
) -> Result<Config, UpgradeError> {
    let to_version = if package_version.major == 0 && package_version.minor == 10 {
        package_version.clone()
    } else {
        Version::new(0, 10, 0)
    };

    print_start_upgrade(&config_version, &to_version);

    if config.get_validator_mixnet_contract_address() != MISSING_VALUE {
        return Err((to_version, "existing config seems to have specified mixnet contract address which was only introduced in 0.10.0! Can't perform upgrade.".to_string()));
    }

    println!(
        "Setting validator REST endpoint to {:?}",
        default_validator_rest_endpoints()
    );

    println!(
        "Setting mixnet contract address to {}",
        DEFAULT_MIXNET_CONTRACT_ADDRESS
    );

    let upgraded_config = config
        .with_custom_version(to_version.to_string().as_ref())
        .with_custom_validators(default_validator_rest_endpoints())
        .with_custom_mixnet_contract(DEFAULT_MIXNET_CONTRACT_ADDRESS);

    upgraded_config.save_to_file(None).map_err(|err| {
        (
            to_version.clone(),
            format!("failed to overwrite config file! - {:?}", err),
        )
    })?;

    print_successful_upgrade(config_version, to_version);

    Ok(upgraded_config)
}

fn patch_0_10_1_upgrade(
    config: Config,
    _matches: &ArgMatches,
    config_version: &Version,
    package_version: &Version,
) -> Result<Config, UpgradeError> {
    // welp, stuff like ports are mostly hardcoded and not part of the config so all is changes is just the version
    // number
    let to_version = package_version;

    print_start_upgrade(&config_version, &to_version);

    let upgraded_config = config.with_custom_version(to_version.to_string().as_ref());

    upgraded_config.save_to_file(None).map_err(|err| {
        (
            to_version.clone(),
            format!("failed to overwrite config file! - {:?}", err),
        )
    })?;

    print_successful_upgrade(config_version, to_version);

    Ok(upgraded_config)
}

fn minor_0_11_upgrade(
    config: Config,
    _matches: &ArgMatches,
    config_version: &Version,
    package_version: &Version,
) -> Result<Config, UpgradeError> {
    let to_version = package_version;

    print_start_upgrade(&config_version, &to_version);

    println!(
        "Setting validator REST endpoint to {:?}",
        default_validator_rest_endpoints()
    );

    println!(
        "Setting mixnet contract address to {}",
        DEFAULT_MIXNET_CONTRACT_ADDRESS
    );

    let upgraded_config = config
        .with_custom_version(to_version.to_string().as_ref())
        .with_custom_validators(default_validator_rest_endpoints())
        .with_custom_mixnet_contract(DEFAULT_MIXNET_CONTRACT_ADDRESS);

    upgraded_config.save_to_file(None).map_err(|err| {
        (
            to_version.clone(),
            format!("failed to overwrite config file! - {:?}", err),
        )
    })?;

    print_successful_upgrade(config_version, to_version);

    Ok(upgraded_config)
}

// TODO: to be renamed once the release version is decided (so presumably either 0.10.2 or 0.11.0)
fn undetermined_version_upgrade(
    config: Config,
    _matches: &ArgMatches,
    config_version: &Version,
    package_version: &Version,
) -> Result<Config, UpgradeError> {
    // If we decide this version should be tagged with 0.11.0, then the following code will be used instead:
    // let to_version = if package_version.major == 0 && package_version.minor == 11 {
    //     package_version.clone()
    // } else {
    //     Version::new(0, 11, 0)
    // };
    let to_version = package_version;
    let id = config.get_id();
    let config_path = Config::default_config_directory(Some(&id));

    #[derive(Deserialize)]
    struct OldNodeDescription {
        name: String,
        description: String,
        link: String,
    }

    print_start_upgrade(&config_version, &to_version);

    let description_file_path: PathBuf = [config_path.to_str().unwrap(), DESCRIPTION_FILE]
        .iter()
        .collect();
    // If the description file already exists, upgrade it
    let new_description = if description_file_path.is_file() {
        let description_content =
            fs::read_to_string(description_file_path.clone()).map_err(|err| {
                (
                    to_version.clone(),
                    format!("failed to read description file! - {:?}", err),
                )
            })?;
        let old_description: OldNodeDescription =
            toml::from_str(&description_content).map_err(|err| {
                (
                    to_version.clone(),
                    format!("failed to deserialize description content! - {:?}", err),
                )
            })?;
        Some(NodeDescription {
            name: old_description.name,
            description: old_description.description,
            link: old_description.link,
            ..Default::default()
        })
    } else {
        None
    };

    let current_annnounce_addr = config.get_announce_address();
    // try to parse it as socket address directly
    let (announce_address, custom_mix_port) = match SocketAddr::from_str(&current_annnounce_addr) {
        Ok(addr) => (addr.ip().to_string(), addr.port()),
        Err(_) => {
            let announce_split = current_annnounce_addr.split(':').collect::<Vec<_>>();
            if announce_split.len() != 2 {
                return Err((
                    to_version.clone(),
                    "failed to correctly parse current announce host".to_string(),
                ));
            }
            (
                announce_split[0].to_string(),
                announce_split[1].parse().unwrap(),
            )
        }
    };

    let upgraded_config = config
        .with_custom_version(to_version.to_string().as_ref())
        .with_announce_address(announce_address)
        .with_mix_port(custom_mix_port);

    if let Some(new_description) = new_description {
        NodeDescription::save_to_file(&new_description, config_path).map_err(|err| {
            (
                to_version.clone(),
                format!("failed to overwrite description file! - {:?}", err),
            )
        })?;
    }

    upgraded_config.save_to_file(None).map_err(|err| {
        (
            to_version.clone(),
            format!("failed to overwrite config file! - {:?}", err),
        )
    })?;

    print_successful_upgrade(config_version, to_version);

    Ok(upgraded_config)
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
                9 => minor_0_10_upgrade(config, matches, &config_version, &Version::new(0, 10, 0)),
                10 => match config_version.patch {
                    0 => patch_0_10_1_upgrade(
                        config,
                        matches,
                        &config_version,
                        &Version::new(0, 10, 1),
                    ),
                    1 => minor_0_11_upgrade(
                        config,
                        matches,
                        &config_version,
                        &Version::new(0, 11, 0),
                    ),
                    _ => undetermined_version_upgrade(
                        config,
                        matches,
                        &config_version,
                        &package_version,
                    ),
                },
                _ => unsupported_upgrade(config_version, package_version),
            },
            _ => unsupported_upgrade(config_version, package_version),
        }
        .unwrap_or_else(|(to_version, err)| {
            eprintln!("{:?}", err);
            print_failed_upgrade(&config_version, &to_version);
            process::exit(1);
        });
    }
}

pub fn execute(matches: &ArgMatches) {
    let package_version = parse_package_version();

    let id = matches.value_of(ID_ARG_NAME).unwrap();

    let mut existing_config = Config::load_from_file(Some(id)).unwrap_or_else(|err| {
        eprintln!("failed to load existing config file! - {:?}", err);
        process::exit(1)
    });

    // versions fields were added in 0.9.0
    if existing_config.get_version() == missing_string_value::<String>() {
        let self_reported_version =
            matches
                .value_of(CURRENT_VERSION_ARG_NAME)
                .unwrap_or_else(|| {
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

#[cfg(test)]
mod upgrade_tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_0_10_2_upgrade() {
        let config = Config::default()
            .with_id("-42")
            .with_announce_address("127.0.0.1:1234")
            .with_custom_version("0.10.1");
        let matches = ArgMatches::default();
        let old_version = Version::new(0, 10, 1);
        let new_version = Version::new(0, 10, 2);
        let new_config =
            undetermined_version_upgrade(config, &matches, &old_version, &new_version).unwrap();
        assert_eq!(new_config.get_version(), "0.10.2");
    }

    #[test]
    #[serial]
    fn test_0_10_2_upgrade_error() {
        let config = Config::default()
            .with_id("-42")
            .with_announce_address("127.0.0.1:1234")
            .with_custom_version("0.10.1");
        let matches = ArgMatches::default();
        let old_version = Version::new(0, 10, 1);
        let new_version = Version::new(0, 10, 2);
        config.save_to_file(None).unwrap();
        let config_file = config.get_config_file_save_location();
        let initial_perms = fs::metadata(config_file.clone()).unwrap().permissions();
        let mut new_perms = initial_perms.clone();
        new_perms.set_readonly(true);
        fs::set_permissions(config_file.clone(), new_perms).unwrap();
        let ret = undetermined_version_upgrade(config, &matches, &old_version, &new_version);
        fs::set_permissions(config_file, initial_perms).unwrap();
        assert!(ret.is_err());
    }
}
