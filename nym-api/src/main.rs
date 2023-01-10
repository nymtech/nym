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
use clap::ArgMatches;
use clap::Parser;
#[cfg(feature = "coconut")]
use coconut::{
    comm::QueryCommunicationChannel,
    dkg::controller::{init_keypair, DkgController},
    InternalSignRequest,
};
use config::NymConfig;
use log::info;
use logging::setup_logging;
use node_status_api::NodeStatusCache;
use nym_contract_cache::cache::NymContractCache;
#[cfg(feature = "coconut")]
use rand::rngs::OsRng;
use std::path::PathBuf;
use std::process;
use std::str::FromStr;
use std::sync::Arc;
use support::{http, nyxd, process_runner};
use task::{wait_for_signal, TaskManager};
use tokio::sync::Notify;
#[cfg(feature = "coconut")]
use url::Url;

mod circulating_supply_api;
mod epoch_operations;
mod network_monitor;
pub(crate) mod node_status_api;
pub(crate) mod nym_contract_cache;
pub(crate) mod support;

#[cfg(feature = "coconut")]
mod coconut;

#[tokio::main]
async fn main() -> Result<()> {
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

async fn run_nym_api(cli_args: CliArgs) -> Result<()> {
    let system_version = clap::crate_version!();
    let save_to_file = cli_args.save_config;
    let config = cli::build_config(cli_args)?;

    // if we just wanted to write data to the config, exit
    if save_to_file {
        info!("Saving the configuration to a file");
        config.save_to_file(None)?;
        return Ok(());
    }

    let mix_denom = std::env::var(MIX_DENOM)?;

    // TODO: under some conditions you HAVE TO create a query client instead
    let signing_nyxd_client = nyxd::Client::new_signing(&config);
    let liftoff_notify = Arc::new(Notify::new());

    // We need a bigger timeout
    let shutdown = TaskManager::new(10);

    #[cfg(feature = "coconut")]
    let coconut_keypair = coconut::keypair::KeyPair::new();

    // let's build our rocket!
    let rocket = http::setup_rocket(
        &config,
        mix_denom,
        Arc::clone(&liftoff_notify),
        signing_nyxd_client.clone(),
        #[cfg(feature = "coconut")]
        coconut_keypair.clone(),
    )
    .await?;

    let nym_contract_cache_state = rocket.state::<NymContractCache>().unwrap().clone();
    let node_status_cache_state = rocket.state::<NodeStatusCache>().unwrap().clone();
    let circulating_supply_cache_state = rocket.state::<CirculatingSupplyCache>().unwrap().clone();

    #[cfg(feature = "coconut")]
    {
        let dkg_controller =
            DkgController::new(&config, signing_nyxd_client.clone(), coconut_keypair, OsRng)
                .await?;
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { dkg_controller.run(shutdown_listener).await });
    }

    // if network monitor is disabled, we're not going to be sending any rewarding hence
    // we're not starting signing client
    let nym_contract_cache_listener = if config.get_network_monitor_enabled() {
        nym_contract_cache::start_with_signing(
            &rocket,
            &shutdown,
            &signing_nyxd_client,
            &config,
            &nym_contract_cache_state,
        )
        .await?
    } else {
        nym_contract_cache::start_without_signing(&config, &nym_contract_cache_state, &shutdown)
    };

    node_status_api::start_cache_refresh(
        &rocket,
        node_status_cache_state,
        &config,
        nym_contract_cache_state,
        nym_contract_cache_listener,
        &shutdown,
    );

    circulating_supply_api::start_cache_refresh(
        &config,
        &circulating_supply_cache_state,
        &shutdown,
    );

    let monitor_builder = network_monitor::setup(
        &config,
        signing_nyxd_client.clone(),
        system_version,
        &rocket,
    );

    // Rocket handles shutdown on its own, but its shutdown handling should be incorporated
    // with that of the rest of the tasks. Currently its runtime is forcefully terminated once
    // nym-api exits.
    let shutdown_handle = rocket.shutdown();

    // Launch the rocket!
    tokio::spawn(rocket.launch());

    // to finish building our monitor, we need to have rocket up and running so that we could
    // obtain our bandwidth credential
    if let Some(monitor_builder) = monitor_builder {
        info!("Starting network monitor...");
        // wait for rocket's liftoff stage
        liftoff_notify.notified().await;

        // we're ready to go! spawn the network monitor!
        let runnables = monitor_builder.build().await;
        runnables.spawn_tasks(&shutdown);
    } else {
        info!("Network monitoring is disabled.");
    }

    process_runner::wait_for_interrupt(shutdown).await;
    shutdown_handle.notify();

    Ok(())
}
