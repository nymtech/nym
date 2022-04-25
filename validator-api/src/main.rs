// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use crate::config::Config;
use crate::contract_cache::ValidatorCacheRefresher;
use crate::network_monitor::NetworkMonitorBuilder;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
use crate::nymd_client::Client;
use crate::storage::ValidatorApiStorage;
use ::config::NymConfig;
use anyhow::Result;
use clap::{crate_version, App, Arg, ArgMatches};
use contract_cache::ValidatorCache;
use log::{info, warn};
use rocket::fairing::AdHoc;
use rocket::http::Method;
use rocket::{Ignite, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors};
use std::process;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use url::Url;
// use validator_client::nymd::SigningNymdClient;
// use validator_client::ValidatorClientError;

use crate::rewarded_set_updater::RewardedSetUpdater;
#[cfg(feature = "coconut")]
use coconut::InternalSignRequest;
use validator_client::nymd::SigningNymdClient;

pub(crate) mod config;
pub(crate) mod contract_cache;
mod network_monitor;
mod node_status_api;
pub(crate) mod nymd_client;
mod rewarded_set_updater;
pub(crate) mod storage;

#[cfg(feature = "coconut")]
mod coconut;

const MONITORING_ENABLED: &str = "enable-monitor";
const REWARDING_ENABLED: &str = "enable-rewarding";
const MIXNET_CONTRACT_ARG: &str = "mixnet-contract";
const MNEMONIC_ARG: &str = "mnemonic";
const WRITE_CONFIG_ARG: &str = "save-config";
const NYMD_VALIDATOR_ARG: &str = "nymd-validator";
const API_VALIDATORS_ARG: &str = "api-validators";
const TESTNET_MODE_ARG_NAME: &str = "testnet-mode";

#[cfg(feature = "coconut")]
const KEYPAIR_ARG: &str = "keypair";

#[cfg(feature = "coconut")]
const COCONUT_ENABLED: &str = "enable-coconut";

#[cfg(not(feature = "coconut"))]
const ETH_ENDPOINT: &str = "eth_endpoint";
#[cfg(not(feature = "coconut"))]
const ETH_PRIVATE_KEY: &str = "eth_private_key";

const REWARDING_MONITOR_THRESHOLD_ARG: &str = "monitor-threshold";

const MIN_MIXNODE_RELIABILITY_ARG: &str = "min_mixnode_reliability";
const MIN_GATEWAY_RELIABILITY_ARG: &str = "min_gateway_reliability";

fn parse_validators(raw: &str) -> Vec<Url> {
    raw.split(',')
        .map(|raw_validator| {
            raw_validator
                .trim()
                .parse()
                .expect("one of the provided validator api urls is invalid")
        })
        .collect()
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
        env!("VERGEN_CARGO_PROFILE"),
    )
}

fn parse_args<'a>() -> ArgMatches<'a> {
    let build_details = long_version();
    let base_app = App::new("Nym Validator API")
        .version(crate_version!())
        .long_version(&*build_details)
        .author("Nymtech")
        .arg(
            Arg::with_name(MONITORING_ENABLED)
                .help("specifies whether a network monitoring is enabled on this API")
                .long(MONITORING_ENABLED)
                .short("m")
        )
        .arg(
            Arg::with_name(REWARDING_ENABLED)
                .help("specifies whether a network rewarding is enabled on this API")
                .long(REWARDING_ENABLED)
                .short("r")
                .requires_all(&[MONITORING_ENABLED, MNEMONIC_ARG])
        )
        .arg(
            Arg::with_name(NYMD_VALIDATOR_ARG)
                .help("Endpoint to nymd part of the validator from which the monitor will grab nodes to test")
                .long(NYMD_VALIDATOR_ARG)
                .takes_value(true)
        )
        .arg(Arg::with_name(MIXNET_CONTRACT_ARG)
                 .long(MIXNET_CONTRACT_ARG)
                 .help("Address of the validator contract managing the network")
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
                .short("w")
        )
        .arg(
            Arg::with_name(API_VALIDATORS_ARG)
                .help("specifies list of all validators on the network issuing coconut credentials. Ensure they are properly ordered")
                .long(API_VALIDATORS_ARG)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(REWARDING_MONITOR_THRESHOLD_ARG)
                .help("Specifies the minimum percentage of monitor test run data present in order to distribute rewards for given interval.")
                .takes_value(true)
                .long(REWARDING_MONITOR_THRESHOLD_ARG)
        )
        .arg(
            Arg::with_name(TESTNET_MODE_ARG_NAME)
                .long(TESTNET_MODE_ARG_NAME)
                .help("Set this validator api to work in a testnet mode that would attempt to use gateway without bandwidth credential requirement")
        );

    #[cfg(feature = "coconut")]
    let base_app = base_app
        .arg(
            Arg::with_name(KEYPAIR_ARG)
                .help("Path to the secret key file")
                .takes_value(true)
                .long(KEYPAIR_ARG),
        )
        .arg(
            Arg::with_name(COCONUT_ENABLED)
                .help("Flag to indicate whether coconut signer authority is enabled on this API")
                .requires_all(&[KEYPAIR_ARG, MNEMONIC_ARG])
                .long(COCONUT_ENABLED),
        );

    #[cfg(not(feature = "coconut"))]
        let base_app = base_app.arg(
        Arg::with_name(ETH_ENDPOINT)
            .help("URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20 tokens")
            .takes_value(true)
            .long(ETH_ENDPOINT),
    ).arg(
        Arg::with_name(ETH_PRIVATE_KEY)
            .help("Ethereum private key used for obtaining bandwidth tokens from ERC20 tokens")
            .takes_value(true)
            .long(ETH_PRIVATE_KEY),
    );

    base_app.get_matches()
}

async fn wait_for_interrupt() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        error!(
            "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
            e
        );
    }
    println!("Received SIGINT - the network monitor will terminate now");
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .filter_module("sled", log::LevelFilter::Warn)
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .filter_module("_", log::LevelFilter::Warn)
        .filter_module("rocket::server", log::LevelFilter::Warn)
        .init();
}

fn override_config(mut config: Config, matches: &ArgMatches<'_>) -> Config {
    if matches.is_present(MONITORING_ENABLED) {
        config = config.with_network_monitor_enabled(true)
    }

    if matches.is_present(REWARDING_ENABLED) {
        config = config.with_rewarding_enabled(true)
    }

    #[cfg(feature = "coconut")]
    if matches.is_present(COCONUT_ENABLED) {
        config = config.with_coconut_signer_enabled(true)
    }

    if let Some(raw_validators) = matches.value_of(API_VALIDATORS_ARG) {
        config = config.with_custom_validator_apis(parse_validators(raw_validators));
    }

    if let Some(raw_validator) = matches.value_of(NYMD_VALIDATOR_ARG) {
        let parsed = match raw_validator.parse() {
            Err(err) => {
                error!("Passed validator argument is invalid - {}", err);
                process::exit(1)
            }
            Ok(url) => url,
        };
        config = config.with_custom_nymd_validator(parsed);
    }

    if let Some(mixnet_contract) = matches.value_of(MIXNET_CONTRACT_ARG) {
        config = config.with_custom_mixnet_contract(mixnet_contract)
    }

    if let Some(mnemonic) = matches.value_of(MNEMONIC_ARG) {
        config = config.with_mnemonic(mnemonic)
    }

    if let Some(monitor_threshold) = matches
        .value_of(REWARDING_MONITOR_THRESHOLD_ARG)
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
        .value_of(MIN_MIXNODE_RELIABILITY_ARG)
        .map(|t| t.parse::<u8>())
    {
        config = config.with_min_mixnode_reliability(
            reliability.expect("Provided reliability is not a u8 number!"),
        )
    }

    if let Some(reliability) = matches
        .value_of(MIN_GATEWAY_RELIABILITY_ARG)
        .map(|t| t.parse::<u8>())
    {
        config = config.with_min_gateway_reliability(
            reliability.expect("Provided reliability is not a u8 number!"),
        )
    }

    #[cfg(feature = "coconut")]
    if let Some(keypair_path) = matches.value_of(KEYPAIR_ARG) {
        let keypair_bs58 = std::fs::read_to_string(keypair_path)
            .unwrap()
            .trim()
            .to_string();
        config = config.with_keypair(keypair_bs58)
    }

    #[cfg(not(feature = "coconut"))]
    if let Some(eth_private_key) = matches.value_of("eth_private_key") {
        config = config.with_eth_private_key(String::from(eth_private_key));
    }

    #[cfg(not(feature = "coconut"))]
    if let Some(eth_endpoint) = matches.value_of("eth_endpoint") {
        config = config.with_eth_endpoint(String::from(eth_endpoint));
    }

    if matches.is_present(TESTNET_MODE_ARG_NAME) {
        config = config.with_testnet_mode(true)
    }

    if matches.is_present(WRITE_CONFIG_ARG) {
        info!("Saving the configuration to a file");
        if let Err(err) = config.save_to_file(None) {
            error!("Failed to write config to a file - {}", err);
            process::exit(1)
        }
    }

    config
}

fn setup_cors() -> Result<Cors> {
    let allowed_origins = AllowedOrigins::all();

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Post, Method::Get]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()?;

    Ok(cors)
}

fn setup_liftoff_notify(notify: Arc<Notify>) -> AdHoc {
    AdHoc::on_liftoff("Liftoff notifier", |_| {
        Box::pin(async move { notify.notify_one() })
    })
}

fn setup_network_monitor<'a>(
    config: &'a Config,
    system_version: &str,
    rocket: &Rocket<Ignite>,
) -> Option<NetworkMonitorBuilder<'a>> {
    if !config.get_network_monitor_enabled() {
        return None;
    }

    // get instances of managed states
    let node_status_storage = rocket.state::<ValidatorApiStorage>().unwrap().clone();
    let validator_cache = rocket.state::<ValidatorCache>().unwrap().clone();

    Some(NetworkMonitorBuilder::new(
        config,
        system_version,
        node_status_storage,
        validator_cache,
    ))
}

// TODO: Remove if still unused
#[allow(dead_code)]
fn expected_monitor_test_runs(config: &Config, interval_length: Duration) -> usize {
    let test_delay = config.get_network_monitor_run_interval();

    // this is just a rough estimate. In real world there will be slightly fewer test runs
    // as they are not instantaneous and hence do not happen exactly every test_delay
    (interval_length.as_secs() / test_delay.as_secs()) as usize
}

async fn setup_rocket(
    config: &Config,
    liftoff_notify: Arc<Notify>,
    _nymd_client: Option<Client<SigningNymdClient>>,
) -> Result<Rocket<Ignite>> {
    // let's build our rocket!
    let rocket = rocket::build()
        .attach(setup_cors()?)
        .attach(setup_liftoff_notify(liftoff_notify))
        .attach(ValidatorCache::stage());

    // This is not a very nice approach. A lazy value would be more suitable, but that's still
    // a nightly feature: https://github.com/rust-lang/rust/issues/74465
    let storage = if cfg!(feature = "coconut") || config.get_network_monitor_enabled() {
        Some(ValidatorApiStorage::init(config.get_node_status_api_database_path()).await?)
    } else {
        None
    };

    #[cfg(feature = "coconut")]
    let rocket = if config.get_coconut_signer_enabled() {
        rocket.attach(InternalSignRequest::stage(
            _nymd_client.expect("Should have a signing client here"),
            config.keypair(),
            storage.clone().unwrap(),
        ))
    } else {
        rocket
    };

    // see if we should start up network monitor and if so, attach the node status api
    if config.get_network_monitor_enabled() {
        Ok(rocket
            .attach(storage::ValidatorApiStorage::stage(storage.unwrap()))
            .attach(node_status_api::stage_full())
            .ignite()
            .await?)
    } else {
        Ok(rocket
            .attach(node_status_api::stage_minimal())
            .ignite()
            .await?)
    }
}

async fn run_validator_api(matches: ArgMatches<'static>) -> Result<()> {
    let system_version = env!("CARGO_PKG_VERSION");

    // try to load config from the file, if it doesn't exist, use default values
    let config = match Config::load_from_file(None) {
        Ok(cfg) => cfg,
        Err(_) => {
            let config_path = Config::default_config_file_path(None)
                .into_os_string()
                .into_string()
                .unwrap();
            warn!(
                "Could not load the configuration file from {}. Either the file did not exist or was malformed. Using the default values instead",
                config_path
            );
            Config::new()
        }
    };

    let config = override_config(config, &matches);
    // if we just wanted to write data to the config, exit
    if matches.is_present(WRITE_CONFIG_ARG) {
        return Ok(());
    }

    let signing_nymd_client = if matches.is_present(MNEMONIC_ARG) {
        Some(Client::new_signing(&config))
    } else {
        None
    };

    let liftoff_notify = Arc::new(Notify::new());

    // let's build our rocket!
    let rocket = setup_rocket(
        &config,
        Arc::clone(&liftoff_notify),
        signing_nymd_client.clone(),
    )
    .await?;
    let monitor_builder = setup_network_monitor(&config, system_version, &rocket);

    let validator_cache = rocket.state::<ValidatorCache>().unwrap().clone();

    // if network monitor is disabled, we're not going to be sending any rewarding hence
    // we're not starting signing client
    if config.get_network_monitor_enabled() {
        let nymd_client = signing_nymd_client.expect("We should have a signing client here");
        let validator_cache_refresher = ValidatorCacheRefresher::new(
            nymd_client.clone(),
            config.get_caching_interval(),
            validator_cache.clone(),
        );

        // spawn our cacher
        tokio::spawn(async move { validator_cache_refresher.run().await });

        // setup our daily uptime updater. Note that if network monitor is disabled, then we have
        // no data for the updates and hence we don't need to start it up
        let storage = rocket.state::<ValidatorApiStorage>().unwrap().clone();
        let uptime_updater = HistoricalUptimeUpdater::new(storage.clone());
        tokio::spawn(async move { uptime_updater.run().await });

        let mut rewarded_set_updater =
            RewardedSetUpdater::new(nymd_client, validator_cache.clone(), storage).await?;

        // spawn rewarded set updater
        tokio::spawn(async move { rewarded_set_updater.run().await.unwrap() });
    } else {
        let nymd_client = Client::new_query(&config);
        let validator_cache_refresher = ValidatorCacheRefresher::new(
            nymd_client,
            config.get_caching_interval(),
            validator_cache,
        );

        // spawn our cacher
        tokio::spawn(async move { validator_cache_refresher.run().await });
    }

    // launch the rocket!
    let shutdown_handle = rocket.shutdown();
    tokio::spawn(rocket.launch());

    // to finish building our monitor, we need to have rocket up and running so that we could
    // obtain our bandwidth credential
    if let Some(monitor_builder) = monitor_builder {
        info!("Starting network monitor...");
        // wait for rocket's liftoff stage
        liftoff_notify.notified().await;

        // we're ready to go! spawn the network monitor!
        let runnables = monitor_builder.build().await;
        runnables.spawn_tasks();
    } else {
        info!("Network monitoring is disabled.");
    }

    wait_for_interrupt().await;
    shutdown_handle.notify();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting validator api...");

    cfg_if::cfg_if! {if #[cfg(feature = "console-subscriber")] {
        // instriment tokio console subscriber needs RUSTFLAGS="--cfg tokio_unstable" at build time
        console_subscriber::init();
    }}

    setup_logging();
    let args = parse_args();
    run_validator_api(args).await
}
