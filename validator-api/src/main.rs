// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use crate::cache::ValidatorCacheRefresher;
use crate::config::Config;
use crate::network_monitor::tested_network::good_topology::parse_topology_file;
use crate::network_monitor::{new_monitor_runnables, NetworkMonitorRunnables};
use crate::nymd_client::Client;
use crate::storage::NodeStatusStorage;
use ::config::{defaults::DEFAULT_VALIDATOR_API_PORT, NymConfig};
use anyhow::Result;
use cache::ValidatorCache;
use clap::{App, Arg, ArgMatches};
use log::info;
use rocket::http::Method;
use rocket::{Build, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors};
use std::process;

pub(crate) mod cache;
pub(crate) mod config;
mod network_monitor;
mod node_status_api;
pub(crate) mod nymd_client;
pub(crate) mod storage;

const MONITORING_ENABLED: &str = "enable-monitor";
const V4_TOPOLOGY_ARG: &str = "v4-topology-filepath";
const V6_TOPOLOGY_ARG: &str = "v6-topology-filepath";
const VALIDATORS_ARG: &str = "validators";
const DETAILED_REPORT_ARG: &str = "detailed-report";
const MIXNET_CONTRACT_ARG: &str = "mixnet-contract";
const MNEMONIC_ARG: &str = "mnemonic";
const WRITE_CONFIG_ARG: &str = "save-config";

pub(crate) const PENALISE_OUTDATED: bool = false;

fn parse_args<'a>() -> ArgMatches<'a> {
    App::new("Nym Network Monitor")
        .author("Nymtech")
        .arg(
            Arg::with_name(MONITORING_ENABLED)
                .help("specifies whether a network monitoring is enabled on this API")
                .long(MONITORING_ENABLED)
        )
        .arg(
            Arg::with_name(V4_TOPOLOGY_ARG)
                .help("location of .json file containing IPv4 'good' network topology")
                .long(V4_TOPOLOGY_ARG)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(V6_TOPOLOGY_ARG)
                .help("location of .json file containing IPv6 'good' network topology")
                .long(V6_TOPOLOGY_ARG)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(VALIDATORS_ARG)
                .help("REST endpoint of the validator the monitor will grab nodes to test")
                .long(VALIDATORS_ARG)
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

    if let Some(v4_topology_path) = matches.value_of(V4_TOPOLOGY_ARG) {
        config = config.with_v4_good_topology(v4_topology_path)
    }

    if let Some(v6_topology_path) = matches.value_of(V4_TOPOLOGY_ARG) {
        config = config.with_v6_good_topology(v6_topology_path)
    }

    if let Some(raw_validator) = matches.value_of(VALIDATORS_ARG) {
        let parsed = match raw_validator.parse() {
            Err(err) => {
                error!("Passed validator argument is invalid - {}", err);
                process::exit(1)
            }
            Ok(url) => url,
        };
        config = config.with_custom_validator(parsed);
    }

    if let Some(mixnet_contract) = matches.value_of(MIXNET_CONTRACT_ARG) {
        config = config.with_custom_mixnet_contract(mixnet_contract)
    }

    if let Some(mnemonic) = matches.value_of(MNEMONIC_ARG) {
        config = config.with_mnemonic(mnemonic)
    }

    if matches.is_present(DETAILED_REPORT_ARG) {
        config = config.detailed_network_monitor_report(true)
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

async fn setup_network_monitor(config: &Config, rocket: &Rocket<Build>) -> NetworkMonitorRunnables {
    // get instances of managed states
    let node_status_storage = rocket.state::<NodeStatusStorage>().unwrap().clone();
    let validator_cache = rocket.state::<ValidatorCache>().unwrap().clone();

    let v4_topology = parse_topology_file(config.get_v4_good_topology_file());
    let v6_topology = parse_topology_file(config.get_v6_good_topology_file());
    network_monitor::check_if_up_to_date(&v4_topology, &v6_topology);

    new_monitor_runnables(
        &config,
        v4_topology,
        v6_topology,
        node_status_storage,
        validator_cache,
    )
}

async fn setup_rocket(config: &Config) -> Result<Rocket<Build>> {
    // let's build our rocket!
    let rocket_config = rocket::config::Config {
        // TODO: probably the port should be configurable?
        port: DEFAULT_VALIDATOR_API_PORT,
        ..Default::default()
    };
    let rocket = rocket::custom(rocket_config)
        .attach(setup_cors()?)
        .attach(ValidatorCache::stage());

    // see if we should start up network monitor and if so, attach the node status api
    if config.get_network_monitor_enabled() {
        Ok(rocket.attach(node_status_api::stage(
            config.get_node_status_api_database_path(),
        )))
    } else {
        Ok(rocket)
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

    // let's build our rocket!
    let rocket = setup_rocket(&config).await?;
    let validator_cache = rocket.state::<ValidatorCache>().unwrap().clone();

    // if network monitor is disabled, we're not going to be sending any rewarding hence
    // we're not starting signing client
    if config.get_network_monitor_enabled() {
        let nymd_client = Client::new_signing(&config);
        let validator_cache_refresher = ValidatorCacheRefresher::new(
            nymd_client,
            config.get_caching_interval(),
            validator_cache,
        );

        // spawn our cacher
        tokio::spawn(async move { validator_cache_refresher.run().await });
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

    if config.get_network_monitor_enabled() {
        info!("Starting network monitor...");
        let runnables = setup_network_monitor(&config, &rocket).await;
        // spawn network monitor!
        runnables.spawn_tasks();
    } else {
        info!("Network monitoring is disabled.");
    }

    // and launch the rocket
    let rocket = rocket.ignite().await.expect("failed to ignite the rocket!");
    let shutdown_handle = rocket.shutdown();

    tokio::spawn(rocket.launch());

    wait_for_interrupt().await;
    shutdown_handle.notify();

    Ok(())
}
