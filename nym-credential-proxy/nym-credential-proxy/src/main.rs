// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod deposits_buffer;
mod quorum_checker;

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        use crate::cli::Cli;
        use clap::Parser;
        use nym_bin_common::bin_info_owned;
        use nym_bin_common::logging::setup_tracing_logger;
        use nym_network_defaults::setup_env;
        use tracing::{info, trace};

        pub mod cli;
        pub mod config;
        pub mod credentials;
        // mod deposit_maker;
        pub mod error;
        pub mod helpers;
        pub mod http;
        pub mod nym_api_helpers;
        pub mod storage;
        pub mod tasks;
        mod webhook;
    }
}

#[cfg(unix)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // std::env::set_var(
    //     "RUST_LOG",
    //     "trace,handlebars=warn,tendermint_rpc=warn,h2=warn,hyper=warn,rustls=warn,reqwest=warn,tungstenite=warn,async_tungstenite=warn,tokio_util=warn,tokio_tungstenite=warn,tokio-util=warn,axum=warn,sqlx-core=warn,nym_validator_client=info",
    // );

    let cli = Cli::parse();
    cli.webhook.ensure_valid_client_url()?;
    trace!("args: {cli:#?}");

    setup_env(cli.config_env_file.as_ref());
    setup_tracing_logger();

    let bin_info = bin_info_owned!();
    info!("using the following version: {bin_info}");

    helpers::run_api(cli).await?;
    Ok(())
}

#[cfg(not(unix))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("This tool is only supported on Unix systems");
    std::process::exit(1)
}
