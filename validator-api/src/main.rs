// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use crate::config::Config;
use crate::network_monitor::new_monitor_runnables;
use crate::network_monitor::tested_network::good_topology::parse_topology_file;
use crate::node_status_api::storage::NodeStatusStorage;
use ::config::NymConfig;
use anyhow::Result;
use cache::ValidatorCache;
use clap::{App, Arg, ArgMatches};
use log::info;
use mixnet_contract::{GatewayBond, MixNodeBond};
use rocket::fairing::AdHoc;
use rocket::http::Method;
use rocket::serde::json::Json;
use rocket::{Build, Rocket, State};
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

mod cache;
pub(crate) mod config;
mod network_monitor;
mod node_status_api;

const MONITORING_ENABLED: &str = "enable-monitor";
const V4_TOPOLOGY_ARG: &str = "v4-topology-filepath";
const V6_TOPOLOGY_ARG: &str = "v6-topology-filepath";
const VALIDATORS_ARG: &str = "validators";
const NODE_STATUS_API_ARG: &str = "node-status-api";
const DETAILED_REPORT_ARG: &str = "detailed-report";
const GATEWAY_SENDING_RATE_ARG: &str = "gateway-rate";
const MIXNET_CONTRACT_ARG: &str = "mixnet-contract";
const CACHE_INTERVAL_ARG: &str = "cache-interval";

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
        .arg(Arg::with_name("mixnet-contract")
                 .long(MIXNET_CONTRACT_ARG)
                 .help("Address of the validator contract managing the network")
                 .takes_value(true),
        )
        .arg(
            Arg::with_name(NODE_STATUS_API_ARG)
                .help("Address of the node status api to submit results to. Most likely it's a local address")
                .long(NODE_STATUS_API_ARG)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(DETAILED_REPORT_ARG)
                .help("specifies whether a detailed report should be printed after each run")
                .long(DETAILED_REPORT_ARG)
        )
        .arg(Arg::with_name(GATEWAY_SENDING_RATE_ARG)
            .help("specifies maximum rate (in packets per second) of test packets being sent to gateway")
            .takes_value(true)
            .long(GATEWAY_SENDING_RATE_ARG)
            .short("r")
        )
        .arg(Arg::with_name(CACHE_INTERVAL_ARG)
        .help("Specified rate, in seconds, at which cache will be refreshed, global for all cache")
        .takes_value(true)
        .long(CACHE_INTERVAL_ARG))
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
    fn parse_validators(raw: &str) -> Vec<String> {
        raw.split(',')
            .map(|raw_validator| raw_validator.trim().into())
            .collect()
    }

    if matches.is_present(MONITORING_ENABLED) {
        config = config.enabled_network_monitor(true)
    }

    if let Some(v4_topology_path) = matches.value_of(V4_TOPOLOGY_ARG) {
        config = config.with_v4_good_topology(v4_topology_path)
    }

    if let Some(v6_topology_path) = matches.value_of(V4_TOPOLOGY_ARG) {
        config = config.with_v6_good_topology(v6_topology_path)
    }

    if let Some(raw_validators) = matches.value_of(VALIDATORS_ARG) {
        config = config.with_custom_validators(parse_validators(raw_validators));
    }

    if let Some(node_status_api_uri) = matches.value_of(NODE_STATUS_API_ARG) {
        config = config.with_custom_node_status_api(node_status_api_uri)
    }

    if let Some(mixnet_contract) = matches.value_of(MIXNET_CONTRACT_ARG) {
        config = config.with_custom_mixnet_contract(mixnet_contract)
    }

    if matches.is_present(DETAILED_REPORT_ARG) {
        config = config.detailed_network_monitor_report(true)
    }

    if let Some(sending_rate) = matches
        .value_of(GATEWAY_SENDING_RATE_ARG)
        .map(|v| v.parse().unwrap())
    {
        config = config.with_gateway_sending_rate(sending_rate)
    }

    if let Some(caching_interval_secs) = matches
        .value_of(CACHE_INTERVAL_ARG)
        .map(|v| v.parse().unwrap())
    {
        config = config.with_caching_interval(Duration::from_secs(caching_interval_secs))
    }

    config
}

#[get("/mixnodes")]
async fn get_mixnodes(cache: &State<Arc<RwLock<ValidatorCache>>>) -> Json<Vec<MixNodeBond>> {
    let cache = cache.read().await;
    Json(cache.mixnodes())
}

#[get("/gateways")]
async fn get_gateways(cache: &State<Arc<RwLock<ValidatorCache>>>) -> Json<Vec<GatewayBond>> {
    let cache = cache.read().await;
    Json(cache.gateways())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();

    println!("Starting validator api...");

    let config = match Config::load_from_file(None) {
        Ok(cfg) => cfg,
        Err(_) => {
            warn!(
                "Configuration file could not be found in {}. Using the default values.",
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

    if config.get_network_monitor_enabled() {
        info!("Network monitor starting...");

        let v4_topology = parse_topology_file(config.get_v4_good_topology_file());
        let v6_topology = parse_topology_file(config.get_v6_good_topology_file());
        network_monitor::check_if_up_to_date(&v4_topology, &v6_topology);

        info!("* validator servers: {:?}", config.get_validators_urls());
        info!(
            "* node status api server: {}",
            config.get_node_status_api_url()
        );
        info!(
            "* mixnet contract: {}",
            config.get_mixnet_contract_address()
        );
        info!(
            "* detailed report printing: {}",
            config.get_detailed_report()
        );
        info!(
            "* gateway sending rate: {} packets/s",
            config.get_gateway_sending_rate()
        );

        let network_monitor_runnables = new_monitor_runnables(&config, v4_topology, v6_topology);
        // network_monitor_runnables.spawn_tasks();
    } else {
        info!("Network monitoring is disabled.")
    }

    async fn init_validator_cache(rocket: Rocket<Build>) -> Rocket<Build> {
        let validator_cache = Arc::new(RwLock::new(ValidatorCache::init(
            vec!["validators_rest_uris".to_string()],
            "mixnet_contract".to_string(),
        )));

        rocket.manage(validator_cache)
    }

    fn stage_validator_cache() -> AdHoc {
        AdHoc::on_ignite("Validator Cache Stage", |rocket| async {
            rocket
                .attach(AdHoc::on_ignite("Init Cache", init_validator_cache))
                .mount("/v1", routes![get_mixnodes, get_gateways])
        })
    }

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

    let rocket = rocket::build()
        .attach(cors)
        .attach(stage_validator_cache())
        .attach(NodeStatusStorage::stage()) // manages state, creates routes, etc
        .ignite()
        .await?;

    let write_mixnode_cache = rocket
        .state::<Arc<RwLock<ValidatorCache>>>()
        .unwrap()
        .clone();

    // tokio::spawn(async move {
    //     let mut interval = time::interval(config.get_caching_interval());
    //     loop {
    //         // TODO: we might end up in a situation where we never update cache because there might
    //         // always be some reader. This should rather use tokio::time::timeout
    //         interval.tick().await;
    //         {
    //             match write_mixnode_cache.try_write() {
    //                 Ok(mut w) => w.cache().await.unwrap(),
    //                 // If we don't get the write lock skip a tick
    //                 Err(e) => error!("Could not aquire write lock on cache: {}", e),
    //             }
    //         }
    //     }
    // });

    let node_status_storage = rocket.state::<NodeStatusStorage>().unwrap().clone();

    tokio::spawn(rocket.launch());

    println!("\n\nwaiting for 5s before adding stuff...\n\n");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // node_status_storage.make_up_mixnode("node3").await;

    node_status_storage.make_up_mixnode("node1").await;
    node_status_storage.make_up_mixnode("node2").await;
    node_status_storage.make_up_mixnode("node3").await;
    node_status_storage.add_up_status("node1").await;

    println!("done");

    wait_for_interrupt().await;

    Ok(())
}
