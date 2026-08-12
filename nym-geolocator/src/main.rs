// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::Args;
use crate::geolocator::Geolocator;
use crate::http::router::build_router;
use crate::http::run_http_server;
use crate::http::state::AppState;
use crate::ip_info_lookup::IpInfoLookup;
use crate::node_scraper::NodeScraper;
use crate::node_scraper::nodes::KnownNodes;
use crate::nyx::nodes::BondedNymNodes;
use crate::nyx::state::OnChainNodes;
use clap::Parser;
use nym_bin_common::bin_info_owned;
use nym_bin_common::logging::setup_tracing_logger;
use nym_network_defaults::{NymNetworkDetails, setup_env};
use nym_task::ShutdownManager;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use std::ops::Deref;
use tracing::info;

pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod error;
pub(crate) mod geolocator;
pub(crate) mod http;
pub(crate) mod ip_info_lookup;
pub(crate) mod node_scraper;
pub(crate) mod nyx;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing_logger();
    let args = Args::parse();
    setup_env(args.config_env_file.as_ref());

    let bin_info = bin_info_owned!();
    info!("using the following version: {bin_info}");

    let config = args.to_config();

    // retrieve initial state from the network
    let network_details = NymNetworkDetails::new_from_env();
    let client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic_and_network_details(
        args.chain.nyxd_addr.as_str(),
        network_details,
        args.chain.mnemonic,
    )?;

    // all nym-nodes that are currently bonded
    info!("building initial state of bonded nym nodes...");
    let bonded_nodes = BondedNymNodes::build_new(&client).await?;
    info!(
        "retrieved {} bonded nym nodes",
        bonded_nodes.read().await.len()
    );

    // all geolocation data submitted on chain by this agent
    info!("building initial state of submitted geolocation data...");
    let on_chain_nodes = OnChainNodes::build_new(&client).await?;
    info!(
        "retrieved {} geolocation data entries",
        on_chain_nodes.get().len()
    );

    // all ips of bonded nodes
    info!("building initial state of self-described node ips... - this could take a while");
    let known_nodes = KnownNodes::build_new(config, bonded_nodes.read().await.deref()).await;
    info!(
        "retrieved self-described data of {} nym nodes",
        known_nodes.len()
    );

    // build the tasks
    let mut shutdown_manager = ShutdownManager::build_new_default()?;

    let described_scraper = NodeScraper::new(config, bonded_nodes.clone(), known_nodes);
    let ip_info_lookup = IpInfoLookup::new(config, args.geolocation.ipinfo_api_token);

    let mut geolocator = Geolocator::new(
        config,
        client,
        bonded_nodes.clone(),
        on_chain_nodes,
        described_scraper,
        ip_info_lookup,
        shutdown_manager.clone_shutdown_token(),
    );

    let http_app_state = AppState { bonded_nodes };
    let http_router = build_router(http_app_state, args.http.http_auth_token)?;
    let http_server_fut = run_http_server(
        http_router,
        args.http.bind_address,
        shutdown_manager.clone_shutdown_token(),
    );

    // 1. start the geolocator
    shutdown_manager.try_spawn_named(async move { geolocator.run().await }, "geolocator");

    // 2. start the http api server
    shutdown_manager.try_spawn_named(http_server_fut, "http-server");

    shutdown_manager.close_tracker();
    shutdown_manager.run_until_shutdown().await;

    Ok(())
}
