// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use crate::cache::ValidatorCacheRefresher;
use crate::config::Config;
use crate::network_monitor::tested_network::good_topology::parse_topology_file;
use crate::network_monitor::{new_monitor_runnables, NetworkMonitorRunnables};
use crate::nymd_client::Client;
use crate::rewarding::Rewarder;
use crate::storage::NodeStatusStorage;
use ::config::{defaults::DEFAULT_VALIDATOR_API_PORT, NymConfig};
use anyhow::Result;
use cache::ValidatorCache;
use clap::{App, Arg, ArgMatches};
use coconut::InternalSignRequest;
use log::{info, warn};
use rocket::http::Method;
use rocket::{Ignite, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors};
use std::process;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use url::Url;
use validator_client::nymd::SigningNymdClient;

pub(crate) mod cache;
mod coconut;
pub(crate) mod config;
mod network_monitor;
mod node_status_api;
pub(crate) mod nymd_client;
mod rewarding;
pub(crate) mod storage;

const MONITORING_ENABLED: &str = "enable-monitor";
const REWARDING_ENABLED: &str = "enable-rewarding";
const V4_TOPOLOGY_ARG: &str = "v4-topology-filepath";
const V6_TOPOLOGY_ARG: &str = "v6-topology-filepath";
const API_VALIDATORS_ARG: &str = "api-validators";
const DETAILED_REPORT_ARG: &str = "detailed-report";
const MIXNET_CONTRACT_ARG: &str = "mixnet-contract";
const MNEMONIC_ARG: &str = "mnemonic";
const WRITE_CONFIG_ARG: &str = "save-config";
const KEYPAIR_ARG: &str = "keypair";
const NYMD_VALIDATOR_ARG: &str = "nymd-validator";

const EPOCH_LENGTH_ARG: &str = "epoch-length";
const FIRST_REWARDING_EPOCH_ARG: &str = "first-epoch";

pub(crate) const PENALISE_OUTDATED: bool = false;

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

fn parse_args<'a>() -> ArgMatches<'a> {
    App::new("Nym Validator API")
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
        )
        .arg(
            Arg::with_name(V4_TOPOLOGY_ARG)
                .help("location of .json file containing IPv4 'good' network topology")
                .long(V4_TOPOLOGY_ARG)
        )
        .arg(
            Arg::with_name(V6_TOPOLOGY_ARG)
                .help("location of .json file containing IPv6 'good' network topology")
                .long(V6_TOPOLOGY_ARG)
                .takes_value(true)
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
                 .takes_value(true),
        )
        .arg(
            Arg::with_name(DETAILED_REPORT_ARG)
                .help("specifies whether a detailed report should be printed after each run")
                .long(DETAILED_REPORT_ARG)
        )
        .arg(
            Arg::with_name(WRITE_CONFIG_ARG)
                .help("specifies whether a config file based on provided arguments should be saved to a file")
                .long(WRITE_CONFIG_ARG)
                .short("w")
        )
        .arg(
            Arg::with_name(KEYPAIR_ARG)
                .help("Path to the secret key file")
                .takes_value(true)
                .long(KEYPAIR_ARG)
        )
        .arg(
            Arg::with_name(FIRST_REWARDING_EPOCH_ARG)
                .help("Datetime specifying beginning of the first rewarding epoch of this length. It must be a valid rfc3339 datetime.")
                .takes_value(true)
                .long(FIRST_REWARDING_EPOCH_ARG)
        )
        .arg(
            Arg::with_name(EPOCH_LENGTH_ARG)
                .help("Length of the current rewarding epoch in hours")
                .takes_value(true)
                .long(EPOCH_LENGTH_ARG)
        )
        .get_matches()
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
        .init();
}

fn override_config(mut config: Config, matches: &ArgMatches) -> Config {
    if matches.is_present(MONITORING_ENABLED) {
        config = config.enabled_network_monitor(true)
    }

    if matches.is_present(REWARDING_ENABLED) {
        config = config.enabled_rewarding(true)
    }

    if let Some(v4_topology_path) = matches.value_of(V4_TOPOLOGY_ARG) {
        config = config.with_v4_good_topology(v4_topology_path)
    }

    if let Some(v6_topology_path) = matches.value_of(V4_TOPOLOGY_ARG) {
        config = config.with_v6_good_topology(v6_topology_path)
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

    if let Some(rewarding_epoch_datetime) = matches.value_of(FIRST_REWARDING_EPOCH_ARG) {
        let first_epoch = OffsetDateTime::parse(rewarding_epoch_datetime, &Rfc3339)
            .expect("Provided first epoch is not a valid rfc3339 datetime!");
        config = config.with_first_rewarding_epoch(first_epoch)
    }

    if let Some(epoch_length) = matches
        .value_of(EPOCH_LENGTH_ARG)
        .map(|len| len.parse::<u64>())
    {
        let epoch_length = epoch_length.expect("Provided epoch length is not a number!");
        config = config.with_epoch_length(Duration::from_secs(epoch_length * 60 * 60));
    }

    if matches.is_present(DETAILED_REPORT_ARG) {
        config = config.detailed_network_monitor_report(true)
    }
    if let Some(keypair_path) = matches.value_of(KEYPAIR_ARG) {
        let keypair_bs58 = std::fs::read_to_string(keypair_path)
            .unwrap()
            .trim()
            .to_string();
        config = config.with_keypair(keypair_bs58)
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

async fn setup_network_monitor(
    config: &Config,
    rocket: &Rocket<Ignite>,
) -> Option<NetworkMonitorRunnables> {
    if !config.get_network_monitor_enabled() {
        return None;
    }

    // get instances of managed states
    let node_status_storage = rocket.state::<NodeStatusStorage>().unwrap().clone();
    let validator_cache = rocket.state::<ValidatorCache>().unwrap().clone();

    let v4_topology = parse_topology_file(config.get_v4_good_topology_file());
    let v6_topology = parse_topology_file(config.get_v6_good_topology_file());
    network_monitor::check_if_up_to_date(&v4_topology, &v6_topology);

    Some(
        new_monitor_runnables(
            config,
            v4_topology,
            v6_topology,
            node_status_storage,
            validator_cache,
        )
        .await,
    )
}

fn setup_rewarder(
    config: &Config,
    rocket: &Rocket<Ignite>,
    nymd_client: &Client<SigningNymdClient>,
) -> Option<Rewarder> {
    if config.get_rewarding_enabled() && config.get_network_monitor_enabled() {
        // get instances of managed states
        let node_status_storage = rocket.state::<NodeStatusStorage>().unwrap().clone();
        let validator_cache = rocket.state::<ValidatorCache>().unwrap().clone();

        Some(Rewarder::new(
            nymd_client.clone(),
            validator_cache,
            node_status_storage,
            config.get_first_rewarding_epoch(),
            config.get_epoch_length(),
        ))
    } else if config.get_rewarding_enabled() {
        warn!("Cannot enable rewarding with the network monitor being disabled");
        None
    } else {
        None
    }
}

async fn setup_rocket(config: &Config) -> Result<Rocket<Ignite>> {
    // let's build our rocket!
    let rocket_config = rocket::config::Config {
        // TODO: probably the port should be configurable?
        port: DEFAULT_VALIDATOR_API_PORT,
        ..Default::default()
    };
    let rocket = rocket::custom(rocket_config)
        .attach(setup_cors()?)
        .attach(ValidatorCache::stage())
        .attach(InternalSignRequest::stage(config.keypair()));

    // see if we should start up network monitor and if so, attach the node status api
    if config.get_network_monitor_enabled() {
        Ok(rocket
            .attach(node_status_api::stage(
                config.get_node_status_api_database_path(),
            ))
            .ignite()
            .await?)
    } else {
        Ok(rocket.ignite().await?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();

    println!("Starting validator api...");

    // try to load config from the file, if it doesn't exist, use default values
    let config = match Config::load_from_file(None) {
        Ok(cfg) => cfg,
        Err(_) => {
            warn!(
                "Configuration file could not be found at {}. Using the default values.",
                Config::default_config_file_path(None)
                    .into_os_string()
                    .into_string()
                    .unwrap()
            );
            Config::new()
        }
    };

    let matches = parse_args();
    let config = override_config(config, &matches);
    // if we just wanted to write data to the config, exit
    if matches.is_present(WRITE_CONFIG_ARG) {
        return Ok(());
    }

    // let's build our rocket!
    let rocket = setup_rocket(&config).await?;
    let monitor_runnables = setup_network_monitor(&config, &rocket).await;

    let validator_cache = rocket.state::<ValidatorCache>().unwrap().clone();

    // if network monitor is disabled, we're not going to be sending any rewarding hence
    // we're not starting signing client
    if config.get_network_monitor_enabled() {
        let nymd_client = Client::new_signing(&config);
        let validator_cache_refresher = ValidatorCacheRefresher::new(
            nymd_client.clone(),
            config.get_caching_interval(),
            validator_cache.clone(),
        );

        // spawn our cacher
        tokio::spawn(async move { validator_cache_refresher.run().await });

        if let Some(rewarder) = setup_rewarder(&config, &rocket, &nymd_client) {
            info!("Periodic rewarding is starting...");
            tokio::spawn(async move { rewarder.run().await });
        } else {
            info!("Periodic rewarding is disabled.");
        }
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

    if let Some(runnables) = monitor_runnables {
        info!("Starting network monitor...");
        // spawn network monitor!
        runnables.spawn_tasks();
    } else {
        info!("Network monitoring is disabled.");
    }

    // and launch the rocket
    let shutdown_handle = rocket.shutdown();

    tokio::spawn(rocket.launch());

    wait_for_interrupt().await;
    shutdown_handle.notify();

    Ok(())
}
