// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use crate::network_monitor::new_monitor_runnables;
use crate::network_monitor::tested_network::good_topology::parse_topology_file;
use crate::node_status_api::storage::NodeStatusStorage;
use anyhow::Result;
use cache::ValidatorCache;
use clap::{App, Arg, ArgMatches};
use log::info;
use mixnet_contract::{GatewayBond, MixNodeBond};
use rocket::http::Method;
use rocket::serde::json::Json;
use rocket::State;
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

mod cache;
mod network_monitor;
mod node_status_api;

const V4_TOPOLOGY_ARG: &str = "v4-topology-filepath";
const V6_TOPOLOGY_ARG: &str = "v6-topology-filepath";
const VALIDATORS_ARG: &str = "validators";
const NODE_STATUS_API_ARG: &str = "node-status-api";
const DETAILED_REPORT_ARG: &str = "detailed-report";
const GATEWAY_SENDING_RATE_ARG: &str = "gateway-rate";
const MIXNET_CONTRACT_ARG: &str = "mixnet-contract";
const CACHE_INTERVAL_ARG: &str = "cache-interval";

const DEFAULT_VALIDATORS: &[&str] = &[
    // "http://testnet-finney-validator.nymtech.net:1317",
    "http://testnet-finney-validator2.nymtech.net:1317",
    "http://mixnet.club:1317",
];

const DEFAULT_NODE_STATUS_API: &str = "http://localhost:8081";
const DEFAULT_GATEWAY_SENDING_RATE: usize = 500;
const DEFAULT_MIXNET_CONTRACT: &str = "hal1k0jntykt7e4g3y88ltc60czgjuqdy4c9c6gv94";
const DEFAULT_CACHE_INTERVAL_ARG: u64 = 60;

pub(crate) const TIME_CHUNK_SIZE: Duration = Duration::from_millis(50);
pub(crate) const PENALISE_OUTDATED: bool = false;

// TODO: let's see how it goes and whether those new adjusting
const MAX_CONCURRENT_GATEWAY_CLIENTS: Option<usize> = Some(50);
const GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_millis(1_500);
pub(crate) const GATEWAY_CONNECTION_TIMEOUT: Duration = Duration::from_millis(2_500);

fn parse_args<'a>() -> ArgMatches<'a> {
    App::new("Nym Network Monitor")
        .author("Nymtech")
        .arg(
            Arg::with_name(V4_TOPOLOGY_ARG)
                .help("location of .json file containing IPv4 'good' network topology")
                .long(V4_TOPOLOGY_ARG)
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(V6_TOPOLOGY_ARG)
                .help("location of .json file containing IPv6 'good' network topology")
                .long(V6_TOPOLOGY_ARG)
                .takes_value(true)
                .required(true),
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
        .help("Specified rate at which cache will be refreshed, global for all cache")
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
    info!("Network monitor starting...");

    let matches = parse_args();
    let v4_topology_path = matches.value_of(V4_TOPOLOGY_ARG).unwrap();
    let v6_topology_path = matches.value_of(V6_TOPOLOGY_ARG).unwrap();

    let v4_topology = parse_topology_file(v4_topology_path);
    let v6_topology = parse_topology_file(v6_topology_path);

    let validators_rest_uris_borrowed = matches
        .values_of(VALIDATORS_ARG)
        .map(|args| args.collect::<Vec<_>>())
        .unwrap_or_else(|| DEFAULT_VALIDATORS.to_vec());

    let validators_rest_uris = validators_rest_uris_borrowed
        .into_iter()
        .map(|uri| uri.to_string())
        .collect::<Vec<_>>();

    let node_status_api_uri = matches
        .value_of(NODE_STATUS_API_ARG)
        .unwrap_or(DEFAULT_NODE_STATUS_API);

    let mixnet_contract = matches
        .value_of(MIXNET_CONTRACT_ARG)
        .unwrap_or(DEFAULT_MIXNET_CONTRACT)
        .to_string();

    let detailed_report = matches.is_present(DETAILED_REPORT_ARG);
    let sending_rate = matches
        .value_of(GATEWAY_SENDING_RATE_ARG)
        .map(|v| v.parse().unwrap())
        .unwrap_or_else(|| DEFAULT_GATEWAY_SENDING_RATE);

    let cache_interval_arg = matches
        .value_of(CACHE_INTERVAL_ARG)
        .map(|v| v.parse().unwrap())
        .unwrap_or_else(|| DEFAULT_CACHE_INTERVAL_ARG);

    network_monitor::check_if_up_to_date(&v4_topology, &v6_topology);
    setup_logging();

    info!("* validator servers: {:?}", validators_rest_uris);
    info!("* node status api server: {}", node_status_api_uri);
    info!("* mixnet contract: {}", mixnet_contract);
    info!("* detailed report printing: {}", detailed_report);
    info!("* gateway sending rate: {} packets/s", sending_rate);

    let network_monitor_runnables = new_monitor_runnables(
        validators_rest_uris.clone(),
        &mixnet_contract,
        &node_status_api_uri,
        v4_topology,
        v6_topology,
        sending_rate,
        detailed_report,
    );

    let mixnode_cache = Arc::new(RwLock::new(ValidatorCache::init(
        vec!["validators_rest_uris".to_string()],
        "mixnet_contract".to_string(),
    )));

    let write_mixnode_cache = Arc::clone(&mixnode_cache);

    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(cache_interval_arg));
        loop {
            interval.tick().await;
            {
                match write_mixnode_cache.try_write() {
                    Ok(mut w) => w.cache().await.unwrap(),
                    // If we don't get the write lock skip a tick
                    Err(e) => error!("Could not aquire write lock on cache: {}", e),
                }
            }
        }
    });

    network_monitor_runnables.spawn_tasks();

    let node_status_storage = NodeStatusStorage::new();

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

    use crate::node_status_api::routes::*;

    rocket::build()
        .attach(cors)
        .mount("/v1", routes![get_mixnodes, get_gateways])
        .mount(
            "/v1/status",
            routes![
                mixnode_report,
                gateway_report,
                mixnodes_full_report,
                gateways_full_report
            ],
        )
        .manage(mixnode_cache)
        .manage(node_status_storage)
        .ignite()
        .await?
        .launch()
        .await?;

    wait_for_interrupt().await;

    Ok(())
}
