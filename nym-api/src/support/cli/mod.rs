pub(crate) mod args;

use ::config::defaults::var_names::{CONFIGURED, MIXNET_CONTRACT_ADDRESS};
use args::{
    CONFIG_ENV_FILE, ENABLED_CREDENTIALS_MODE_ARG_NAME, ID, MIN_GATEWAY_RELIABILITY_ARG,
    MIN_MIXNODE_RELIABILITY_ARG, MIXNET_CONTRACT_ARG, MNEMONIC_ARG, MONITORING_ENABLED,
    NYXD_VALIDATOR_ARG, REWARDING_ENABLED, REWARDING_MONITOR_THRESHOLD_ARG, WRITE_CONFIG_ARG,
};
use clap::{crate_version, App, Arg, ArgMatches};
use config::{defaults::mainnet::read_var_if_not_default, NymConfig};
use std::{fs, process};

#[cfg(feature = "coconut")]
use args::{ANNOUNCE_ADDRESS, COCONUT_ENABLED};

use super::config::Config;

pub fn parse_args() -> ArgMatches {
    let build_details = long_version();
    let base_app = App::new("Nym API")
        .version(crate_version!())
        .long_version(&*build_details)
        .author("Nym")
        .arg(
            Arg::with_name(CONFIG_ENV_FILE)
                .help("Path pointing to an env file that configures the Nym API")
                .long(CONFIG_ENV_FILE)
                .short('c')
                .takes_value(true)
        )
        .arg(
            Arg::with_name(ID)
                .help("Id of the nym-api we want to run")
                .long(ID)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(MONITORING_ENABLED)
                .help("specifies whether a network monitoring is enabled on this API")
                .long(MONITORING_ENABLED)
                .short('m')
        )
        .arg(
            Arg::with_name(REWARDING_ENABLED)
                .help("specifies whether a network rewarding is enabled on this API")
                .long(REWARDING_ENABLED)
                .short('r')
                .requires_all(&[MONITORING_ENABLED, MNEMONIC_ARG])
        )
        .arg(
            Arg::with_name(NYXD_VALIDATOR_ARG)
                .help("Endpoint to nyxd instance from which the monitor will grab nodes to test")
                .long(NYXD_VALIDATOR_ARG)
                .takes_value(true)
        )
        .arg(Arg::with_name(MIXNET_CONTRACT_ARG)
                 .long(MIXNET_CONTRACT_ARG)
                 .help("Address of the mixnet contract managing the network")
                 .takes_value(true),
        )
        .arg(Arg::with_name(MNEMONIC_ARG)
                 .long(MNEMONIC_ARG)
                 .help("Mnemonic of the network monitor used for rewarding operators")
                 .takes_value(true)
        )
        .arg(
            Arg::with_name(WRITE_CONFIG_ARG)
                .help("specifies whether a config file based on provided arguments should be saved to a file")
                .long(WRITE_CONFIG_ARG)
                .short('w')
        )
        .arg(
            Arg::with_name(REWARDING_MONITOR_THRESHOLD_ARG)
                .help("Specifies the minimum percentage of monitor test run data present in order to distribute rewards for given interval.")
                .takes_value(true)
                .long(REWARDING_MONITOR_THRESHOLD_ARG)
        )
        .arg(
            Arg::with_name(MIN_MIXNODE_RELIABILITY_ARG)
                .long(MIN_MIXNODE_RELIABILITY_ARG)
                .help("Mixnodes with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(MIN_GATEWAY_RELIABILITY_ARG)
                .long(MIN_GATEWAY_RELIABILITY_ARG)
                .help("Gateways with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(ENABLED_CREDENTIALS_MODE_ARG_NAME)
                .long(ENABLED_CREDENTIALS_MODE_ARG_NAME)
                .help("Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement")
        );

    #[cfg(feature = "coconut")]
    let base_app = base_app
        .arg(
            Arg::with_name(ANNOUNCE_ADDRESS)
                .help("Announced address where coconut clients will connect.")
                .long(ANNOUNCE_ADDRESS)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(COCONUT_ENABLED)
                .help("Flag to indicate whether coconut signer authority is enabled on this API")
                .requires_all(&[MNEMONIC_ARG, ANNOUNCE_ADDRESS])
                .long(COCONUT_ENABLED),
        );
    base_app.get_matches()
}

fn long_version() -> String {
    format!(
        r#"
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
"#,
        "Build Timestamp:",
        env!("VERGEN_BUILD_TIMESTAMP"),
        "Build Version:",
        env!("VERGEN_BUILD_SEMVER"),
        "Commit SHA:",
        env!("VERGEN_GIT_SHA"),
        "Commit Date:",
        env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        "Commit Branch:",
        env!("VERGEN_GIT_BRANCH"),
        "rustc Version:",
        env!("VERGEN_RUSTC_SEMVER"),
        "rustc Channel:",
        env!("VERGEN_RUSTC_CHANNEL"),
        "cargo Profile:",
        env!("VERGEN_CARGO_PROFILE")
    )
}

pub(crate) fn override_config(mut config: Config, matches: &ArgMatches) -> Config {
    if let Some(id) = matches.value_of(args::ID) {
        fs::create_dir_all(Config::default_config_directory(Some(id)))
            .expect("Could not create config directory");
        fs::create_dir_all(Config::default_data_directory(Some(id)))
            .expect("Could not create data directory");
        config = config.with_id(id);
    }

    if matches.is_present(args::MONITORING_ENABLED) {
        config = config.with_network_monitor_enabled(true)
    }

    if matches.is_present(args::REWARDING_ENABLED) {
        config = config.with_rewarding_enabled(true)
    }

    #[cfg(feature = "coconut")]
    if matches.is_present(args::COCONUT_ENABLED) {
        config = config.with_coconut_signer_enabled(true)
    }

    #[cfg(feature = "coconut")]
    if let Some(announce_address) = matches.value_of(args::ANNOUNCE_ADDRESS) {
        config = config.with_announce_address(
            Url::parse(announce_address).expect("Could not parse announce address"),
        );
    }

    if let Some(raw_validator) = matches.value_of(args::NYXD_VALIDATOR_ARG) {
        let parsed = match raw_validator.parse() {
            Err(err) => {
                error!("Passed validator argument is invalid - {err}");
                process::exit(1)
            }
            Ok(url) => url,
        };
        config = config.with_custom_nyxd_validator(parsed);
    }

    if let Some(mixnet_contract) = matches.value_of(args::MIXNET_CONTRACT_ARG) {
        config = config.with_custom_mixnet_contract(mixnet_contract)
    } else if std::env::var(CONFIGURED).is_ok() {
        if let Some(mixnet_contract) = read_var_if_not_default(MIXNET_CONTRACT_ADDRESS) {
            config = config.with_custom_mixnet_contract(mixnet_contract)
        }
    }

    if let Some(mnemonic) = matches.value_of(args::MNEMONIC_ARG) {
        config = config.with_mnemonic(mnemonic)
    }

    if let Some(monitor_threshold) = matches
        .value_of(args::REWARDING_MONITOR_THRESHOLD_ARG)
        .map(|t| t.parse::<u8>())
    {
        let monitor_threshold =
            monitor_threshold.expect("Provided monitor threshold is not a number!");
        assert!(
            monitor_threshold <= 100,
            "Provided monitor threshold is greater than 100!"
        );
        config = config.with_minimum_interval_monitor_threshold(monitor_threshold)
    }

    if let Some(reliability) = matches
        .value_of(args::MIN_MIXNODE_RELIABILITY_ARG)
        .map(|t| t.parse::<u8>())
    {
        config = config.with_min_mixnode_reliability(
            reliability.expect("Provided reliability is not a u8 number!"),
        )
    }

    if let Some(reliability) = matches
        .value_of(args::MIN_GATEWAY_RELIABILITY_ARG)
        .map(|t| t.parse::<u8>())
    {
        config = config.with_min_gateway_reliability(
            reliability.expect("Provided reliability is not a u8 number!"),
        )
    }

    if matches.is_present(args::ENABLED_CREDENTIALS_MODE_ARG_NAME) {
        config = config.with_disabled_credentials_mode(false)
    }

    if matches.is_present(args::WRITE_CONFIG_ARG) {
        info!("Saving the configuration to a file");
        if let Err(err) = config.save_to_file(None) {
            error!("Failed to write config to a file - {err}");
            process::exit(1)
        }
    }

    config
}
