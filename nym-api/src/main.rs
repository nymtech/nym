// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use crate::support::cli;
use crate::support::cli::CliArgs;
use crate::support::storage;
use ::config::defaults::setup_env;
use ::config::defaults::var_names::MIX_DENOM;
use anyhow::Result;
use circulating_supply_api::cache::CirculatingSupplyCache;
use clap::Parser;
use config::NymConfig;
use log::info;
use logging::setup_logging;
use node_status_api::NodeStatusCache;
use nym_contract_cache::cache::NymContractCache;
use std::error::Error;
use std::sync::Arc;
use support::{http, nyxd};
use task::{wait_for_signal_and_error, TaskManager};
use tokio::sync::Notify;

mod circulating_supply_api;
mod epoch_operations;
mod network_monitor;
pub(crate) mod node_status_api;
pub(crate) mod nym_contract_cache;
pub(crate) mod support;

#[cfg(feature = "coconut")]
use coconut::dkg::controller::DkgController;

use crate::epoch_operations::RewardedSetUpdater;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
use crate::support::config::Config;
use crate::support::storage::NymApiStorage;
#[cfg(feature = "coconut")]
use rand::rngs::OsRng;

#[cfg(feature = "coconut")]
mod coconut;

struct ShutdownHandlers {
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
) -> Result<ShutdownHandlers, Box<dyn Error + Send + Sync>> {
    let system_version = clap::crate_version!();
    let mix_denom = std::env::var(MIX_DENOM)?;

    let nyxd_client = nyxd::Client::new(&config);
    let liftoff_notify = Arc::new(Notify::new());

    // TODO: question to @BN: why are we creating a fresh coconut keypair here every time as opposed
    // to using some persistent keys?
    #[cfg(feature = "coconut")]
    let coconut_keypair = coconut::keypair::KeyPair::new();

    // let's build our rocket!
    let rocket = http::setup_rocket(
        &config,
        mix_denom,
        Arc::clone(&liftoff_notify),
        nyxd_client.clone(),
        #[cfg(feature = "coconut")]
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

    #[cfg(feature = "coconut")]
    {
        let dkg_controller =
            DkgController::new(&config, nyxd_client.clone(), coconut_keypair, OsRng).await?;
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { dkg_controller.run(shutdown_listener).await });
    }

    // only start the uptime updater if the monitoring if it's enabled
    if config.get_network_monitor_enabled() {
        // if network monitor is enabled, the storage MUST BE available
        let storage = maybe_storage.unwrap();

        HistoricalUptimeUpdater::start(storage, &shutdown);

        // the same idea holds for rewarding
        if config.get_rewarding_enabled() {
            RewardedSetUpdater::start(
                nyxd_client.clone(),
                nym_contract_cache_state,
                storage,
                &shutdown,
            );
        }
    }

    let tmp_owned_contract_cache_state = nym_contract_cache_state.to_owned();
    let tmp_owned_storage = maybe_storage.unwrap().to_owned();

    // Launch the rocket!
    tokio::spawn(rocket.launch());

    // finally, to finish building our monitor, we need to have rocket up and running so that we could
    // obtain our bandwidth credential
    if config.get_network_monitor_enabled() {
        let monitor_builder = network_monitor::setup(
            &config,
            tmp_owned_contract_cache_state,
            tmp_owned_storage,
            nyxd_client,
            system_version,
        );
        info!("Starting network monitor...");
        // wait for rocket's liftoff stage
        liftoff_notify.notified().await;

        // we're ready to go! spawn the network monitor!
        let runnables = monitor_builder.build().await;
        runnables.spawn_tasks(&shutdown);
    }

    Ok(ShutdownHandlers {
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

    let mut shutdown_handlers = start_nym_api_tasks(config).await?;

    let res = wait_for_signal_and_error(&mut shutdown_handlers.task_manager_handle).await;
    shutdown_handlers.rocket_handle.notify();

    res
}
