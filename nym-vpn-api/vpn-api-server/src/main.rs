// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use crate::cli::Cli;
use crate::error::VpnApiError;
use crate::http::state::ApiState;
use crate::http::HttpServer;
use crate::storage::VpnApiStorage;
use crate::tasks::StoragePruner;
use clap::Parser;
use nym_bin_common::logging::setup_tracing_logger;
use nym_network_defaults::setup_env;
use tracing::{info, trace};

pub mod cli;
pub mod config;
pub mod credentials;
pub mod error;
pub mod helpers;
pub mod http;
pub mod nym_api_helpers;
pub mod storage;
pub mod tasks;
mod webhook;

pub async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    // if we fail to setup the signals, we should just blow up
    #[allow(clippy::expect_used)]
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM channel");
    #[allow(clippy::expect_used)]
    let mut sigquit = signal(SignalKind::quit()).expect("Failed to setup SIGQUIT channel");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received SIGINT");
        },
        _ = sigterm.recv() => {
            info!("Received SIGTERM");
        }
        _ = sigquit.recv() => {
            info!("Received SIGQUIT");
        }
    }
}

async fn run_api(cli: Cli) -> Result<(), VpnApiError> {
    // create the tasks
    let bind_address = cli.bind_address();

    let storage = VpnApiStorage::init(cli.persistent_storage_path()).await?;
    let mnemonic = cli.mnemonic;
    let auth_token = cli.http_auth_token;
    let webhook_cfg = cli.webhook;
    let api_state = ApiState::new(storage.clone(), webhook_cfg, mnemonic).await?;
    let http_server = HttpServer::new(bind_address, api_state.clone(), auth_token);

    let storage_pruner = StoragePruner::new(api_state.cancellation_token(), storage);

    // spawn all the tasks
    api_state.try_spawn(http_server.run_forever());
    api_state.try_spawn(storage_pruner.run_forever());

    // wait for cancel signal (SIGINT, SIGTERM or SIGQUIT)
    wait_for_signal().await;

    // cancel all the tasks and wait for all task to terminate
    api_state.cancel_and_wait().await;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var(
        "RUST_LOG",
        "trace,handlebars=warn,tendermint_rpc=warn,h2=warn,hyper=warn,rustls=warn,reqwest=warn,tungstenite=warn,async_tungstenite=warn,tokio_util=warn,tokio_tungstenite=warn,tokio-util=warn,nym_validator_client=info",
    );

    let cli = Cli::parse();
    cli.webhook.ensure_valid_client_url()?;
    trace!("args: {cli:#?}");

    setup_env(cli.config_env_file.as_ref());
    setup_tracing_logger();

    run_api(cli).await?;
    Ok(())
}
