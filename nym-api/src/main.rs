// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use crate::support::cli;
use crate::support::storage;
use ::config::defaults::setup_env;
use ::config::defaults::var_names::MIX_DENOM;
use anyhow::Result;
use build_information::BinaryBuildInformation;
use clap::ArgMatches;
#[cfg(feature = "coconut")]
use coconut::{
    comm::QueryCommunicationChannel,
    dkg::controller::{init_keypair, DkgController},
    InternalSignRequest,
};
use log::info;
use logging::setup_logging;
use logging::setup_logging;
use node_status_api::NodeStatusCache;
use nym_contract_cache::cache::NymContractCache;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use support::{http, nyxd, process_runner};
use task::{wait_for_signal, TaskManager};
use tokio::sync::Notify;
#[cfg(feature = "coconut")]
use url::Url;
#[cfg(feature = "coconut")]
use rand::rngs::OsRng;
use support::{http, nyxd, process_runner};
use support::{nyxd, openapi};

mod circulating_supply_api;
mod epoch_operations;
mod network_monitor;
pub(crate) mod node_status_api;
pub(crate) mod nym_contract_cache;
pub(crate) mod support;

#[cfg(feature = "coconut")]
mod coconut;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String =
        BinaryBuildInformation::new(env!("CARGO_PKG_VERSION")).pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

// explicitly defined custom parser (as opposed to just using
// #[arg(value_parser = clap::value_parser!(u8).range(0..100))]
// for better error message
fn threshold_in_range(s: &str) -> Result<u8, String> {
    let threshold: usize = s
        .parse()
        .map_err(|_| format!("`{s}` isn't a valid threshold number"))?;
    if threshold > 100 {
        Err(format!("{threshold} is not within the range 0-100"))
    } else {
        Ok(threshold as u8)
    }
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
struct ApiArgs {
    /// Path pointing to an env file that configures the Nym API.
    #[clap(short, long)]
    config_env_file: Option<std::path::PathBuf>,

    /// Id of the nym-api we want to run
    #[clap(long)]
    id: Option<String>,

    /// Specifies whether network monitoring is enabled on this API
    #[clap(short = 'm', long)]
    enable_monitor: bool,

    /// Specifies whether network rewarding is enabled on this API
    #[clap(short = 'r', long, requires = "enable_monitor", requires = "mnemonic")]
    enable_rewarding: bool,

    /// Endpoint to nyxd instance from which the monitor will grab nodes to test
    #[clap(long)]
    nyxd_validator: Option<url::Url>,

    /// Address of the mixnet contract managing the network
    #[clap(long)]
    mixnet_contract: Option<nyxd::AccountId>,

    /// Mnemonic of the network monitor used for rewarding operators
    // even though we're currently converting the mnemonic to string (and then back to the concrete type)
    // at least we're getting immediate validation when passing the arguments
    #[clap(long)]
    mnemonic: Option<bip39::Mnemonic>,

    /// Specifies whether a config file based on provided arguments should be saved to a file
    #[clap(short = 'w', long)]
    save_config: bool,

    /// Specifies the minimum percentage of monitor test run data present in order to distribute rewards for given interval.
    #[clap(long, value_parser = threshold_in_range)]
    monitor_threshold: Option<u8>,

    /// Mixnodes with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.
    #[clap(long, value_parser = threshold_in_range)]
    min_mixnode_reliability: Option<u8>,

    /// Gateways with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.
    #[clap(long, value_parser = threshold_in_range)]
    min_gateway_reliability: Option<u8>,

    /// Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement
    #[clap(long)]
    enabled_credentials_mode: bool,

    /// Announced address where coconut clients will connect.
    #[cfg(feature = "coconut")]
    #[clap(long)]
    announce_address: Option<url::Url>,

    /// Flag to indicate whether coconut signer authority is enabled on this API
    #[cfg(feature = "coconut")]
    #[clap(long, requires = "mnemonic", requires = "announce-address")]
    enable_coconut: bool,
}

async fn wait_for_interrupt(mut shutdown: TaskManager) {
    wait_for_signal().await;

    log::info!("Sending shutdown");
    shutdown.signal_shutdown().ok();

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting nym api...");

    cfg_if::cfg_if! {if #[cfg(feature = "console-subscriber")] {
        // instrument tokio console subscriber needs RUSTFLAGS="--cfg tokio_unstable" at build time
        console_subscriber::init();
    }}

    setup_logging();
    let args = cli::parse_args();
    let config_env_file = args
        .value_of(cli::args::CONFIG_ENV_FILE)
        .map(|s| PathBuf::from_str(s).expect("invalid env config file"));
    setup_env(config_env_file);
    run_nym_api(args).await
}

async fn run_nym_api(cli_args: ApiArgs) -> Result<()> {
    let (system_version, config) = cli::build_config(cli_args);

    #[cfg(feature = "coconut")]
    if !_already_inited {
        init_keypair(&config)?;
    }

    // if we just wanted to write data to the config, exit
    if save_to_file {
        info!("Saving the configuration to a file");
        if let Err(err) = config.save_to_file(None) {
            error!("Failed to write config to a file - {err}");
            process::exit(1)
        } else {
            return Ok(());
        }
    }

    let mix_denom = std::env::var(MIX_DENOM).expect("mix denom not set");
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
