// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::EpochAdvancer;
use crate::support::cli;
use crate::support::storage;
use ::nym_config::defaults::setup_env;
use clap::Parser;
use mixnet_contract_cache::cache::MixnetContractCache;
use node_status_api::NodeStatusCache;
use support::nyxd;
use tracing::{info, trace};

mod circulating_supply_api;
mod ecash;
mod epoch_operations;
mod key_rotation;
pub(crate) mod mixnet_contract_cache;
pub(crate) mod network;
mod network_monitor;
pub(crate) mod node_describe_cache;
mod node_performance;
pub(crate) mod node_status_api;
pub(crate) mod nym_nodes;
mod signers_cache;
mod status;
pub(crate) mod support;
mod unstable_routes;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    cfg_if::cfg_if! {if #[cfg(feature = "console-subscriber")] {
        // instrument tokio console subscriber needs RUSTFLAGS="--cfg tokio_unstable" at build time
        console_subscriber::init();
    } else {
        nym_bin_common::logging::setup_tracing_logger();
    }}

    info!("Starting nym api...");

    let args = cli::Cli::parse();
    trace!("args: {:#?}", args);

    setup_env(args.config_env_file.as_ref());
    args.execute().await
}
