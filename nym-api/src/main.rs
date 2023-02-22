// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use crate::epoch_operations::RewardedSetUpdater;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
use crate::support::cli;
use crate::support::cli::CliArgs;
use crate::support::config::Config;
use crate::support::storage;
use crate::support::storage::NymApiStorage;
use ::config::defaults::setup_env;
use anyhow::Result;
use circulating_supply_api::cache::CirculatingSupplyCache;
use clap::Parser;
use coconut::dkg::controller::DkgController;
use nym_config::NymConfig;
use log::info;
use node_status_api::NodeStatusCache;
use nym_bin_common::logging::setup_logging;
use nym_contract_cache::cache::NymContractCache;
use nym_task::TaskManager;
use rand::rngs::OsRng;
use std::error::Error;
use support::{http, nyxd};

mod circulating_supply_api;
mod coconut;
mod epoch_operations;
mod network_monitor;
pub(crate) mod node_status_api;
pub(crate) mod nym_contract_cache;
pub(crate) mod support;

struct ShutdownHandles {
    task_manager_handle: TaskManager,
    rocket_handle: rocket::Shutdown,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Starting nym api...");

    cfg_if::cfg_if! {if #[cfg(feature = "console-subscriber")] {
        // instrument tokio console subscriber needs RUSTFLAGS="--cfg tokio_unstable" at build time
        console_subscriber::init();
    }}

    setup_logging();
    let args = cli::CliArgs::parse();
    setup_env(args.config_env_file.as_ref());
    run_nym_api(args).await
}

async fn start_nym_api_tasks(
    config: Config,
) -> Result<ShutdownHandles, Box<dyn Error + Send + Sync>> {
    let system_version = clap::crate_version!();

    let nyxd_client = nyxd::Client::new(&config);
    let mix_denom = nyxd_client.chain_details().await.mix_denom.base;

    let coconut_keypair = coconut::keypair::KeyPair::new();

    // let's build our rocket!
    let rocket = http::setup_rocket(
        &config,
        mix_denom,
        nyxd_client.clone(),
        coconut_keypair.clone(),
    )
    .await?;

    // setup shutdowns
    let shutdown = TaskManager::new(10);

    // Rocket handles shutdown on its own, but its shutdown handling should be incorporated
    // with that of the rest of the tasks. Currently its runtime is forcefully terminated once
    // nym-api exits.
    let rocket_shutdown_handle = rocket.shutdown();

    // get references to the managed state
    let nym_contract_cache_state = rocket.state::<NymContractCache>().unwrap();
    let node_status_cache_state = rocket.state::<NodeStatusCache>().unwrap();
    let circulating_supply_cache_state = rocket.state::<CirculatingSupplyCache>().unwrap();
    let maybe_storage = rocket.state::<NymApiStorage>();

    // start all the caches first
    let nym_contract_cache_listener = nym_contract_cache::start_refresher(
        &config,
        nym_contract_cache_state,
        nyxd_client.clone(),
        &shutdown,
    );
    node_status_api::start_cache_refresh(
        &config,
        nym_contract_cache_state,
        node_status_cache_state,
        maybe_storage,
        nym_contract_cache_listener,
        &shutdown,
    );
    circulating_supply_api::start_cache_refresh(
        &config,
        nyxd_client.clone(),
        circulating_supply_cache_state,
        &shutdown,
    );

    // start dkg task
    if config.get_coconut_signer_enabled() {
        DkgController::start(
            &config,
            nyxd_client.clone(),
            coconut_keypair,
            OsRng,
            &shutdown,
        )
        .await?;
    }

    // and then only start the uptime updater (and the monitor itself, duh)
    // if the monitoring if it's enabled
    if config.get_network_monitor_enabled() {
        // if network monitor is enabled, the storage MUST BE available
        let storage = maybe_storage.unwrap();

        network_monitor::start(
            &config,
            nym_contract_cache_state,
            storage,
            nyxd_client.clone(),
            system_version,
            &shutdown,
        )
        .await;

        HistoricalUptimeUpdater::start(storage, &shutdown);

        // start 'rewarding' if its enabled
        if config.get_rewarding_enabled() {
            epoch_operations::ensure_rewarding_permission(&nyxd_client).await?;
            RewardedSetUpdater::start(nyxd_client, nym_contract_cache_state, storage, &shutdown);
        }
    }

    // Launch the rocket, serve http endpoints and finish the startup
    tokio::spawn(rocket.launch());

    Ok(ShutdownHandles {
        task_manager_handle: shutdown,
        rocket_handle: rocket_shutdown_handle,
    })
}

async fn run_nym_api(cli_args: CliArgs) -> Result<(), Box<dyn Error + Send + Sync>> {
    let save_to_file = cli_args.save_config;
    let config = cli::build_config(cli_args)?;

    // if we just wanted to write data to the config, exit, don't start any tasks
    if save_to_file {
        info!("Saving the configuration to a file");
        config.save_to_file(None)?;
        return Ok(());
    }

    let shutdown_handlers = start_nym_api_tasks(config).await?;

    let res = shutdown_handlers
        .task_manager_handle
        .catch_interrupt()
        .await;
    log::info!("Stopping nym API");
    shutdown_handlers.rocket_handle.notify();

    res
}
