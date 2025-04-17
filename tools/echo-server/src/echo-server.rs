// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use clap::Parser;
use echo_server::NymEchoServer;
use nym_crypto::asymmetric::ed25519;
use tracing::info;

#[derive(Parser, Debug)]
struct Args {
    /// Optional gateway to use
    #[clap(short, long)]
    gateway: Option<ed25519::PublicKey>,

    /// Optional config path to specify
    #[clap(short, long)]
    config_path: Option<String>,

    /// Optional env file - defaults to Mainnet if None
    #[clap(short, long)]
    env: Option<String>,

    /// Listen port
    #[clap(short, long, default_value = "8080")]
    listen_port: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_tracing_logger();
    let args = Args::parse();
    let mut echo_server = NymEchoServer::new(
        args.gateway,
        args.config_path.as_deref(),
        args.env,
        args.listen_port.as_str(),
    )
    .await?;

    let echo_addr = echo_server.nym_address().await;
    info!("listening on {echo_addr}");

    echo_server.run().await?;

    Ok(())
}
