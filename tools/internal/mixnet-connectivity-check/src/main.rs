// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Args, Parser, Subcommand};
use futures::stream::StreamExt;
use nym_bin_common::output_format::OutputFormat;
use nym_bin_common::{bin_info, bin_info_owned};
use nym_crypto::asymmetric::ed25519;
use nym_network_defaults::setup_env;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sdk::{mixnet, DebugConfig};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::time::timeout;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Commands::CheckConnectivity(args) => connectivity_test(args).await?,
            Commands::BuildInfo(args) => build_info(args),
        }
        Ok(())
    }
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Commands {
    /// Attempt to run a simple connectivity test
    CheckConnectivity(ConnectivityArgs),

    /// Show build information of this binary
    BuildInfo(BuildInfoArgs),
}

#[derive(Args, Clone, Debug)]
struct ConnectivityArgs {
    #[clap(long)]
    gateway: Option<ed25519::PublicKey>,

    #[clap(long)]
    ignore_performance: bool,
}

#[derive(clap::Args, Debug)]
pub(crate) struct BuildInfoArgs {
    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

fn build_info(args: BuildInfoArgs) {
    println!("{}", args.output.format(&bin_info_owned!()))
}

async fn connectivity_test(args: ConnectivityArgs) -> anyhow::Result<()> {
    let env = mixnet::NymNetworkDetails::new_from_env();
    let mut debug_config = DebugConfig::default();
    debug_config.cover_traffic.disable_loop_cover_traffic_stream = true;
    debug_config
        .traffic
        .disable_main_poisson_packet_distribution = true;

    if args.ignore_performance {
        debug_config.topology.minimum_mixnode_performance = 0;
        debug_config.topology.minimum_gateway_performance = 0;
    };

    let client_builder = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(env)
        .debug_config(debug_config);

    let mixnet_client = if let Some(gateway) = args.gateway {
        client_builder
            .request_gateway(gateway.to_string())
            .build()?
    } else {
        client_builder.build()?
    };

    print!("connecting to mixnet... ");
    let mut client = match mixnet_client.connect_to_mixnet().await {
        Ok(client) => {
            println!("✅");
            client
        }
        Err(err) => {
            println!("❌");
            println!("failed to connect: {err}");
            return Err(err.into());
        }
    };
    let our_address = client.nym_address();

    println!("attempting to send a message to ourselves ({our_address})");

    client
        .send_plain_message(*our_address, "hello there")
        .await?;

    print!("awaiting response... ");

    match timeout(Duration::from_secs(5), client.next()).await {
        Err(_timeout) => {
            println!("❌");
            println!("timed out while waiting for the response...");
        }
        Ok(Some(received)) => match String::from_utf8(received.message) {
            Ok(message) => {
                println!("✅");
                println!("received '{message}' back!");
            }
            Err(err) => {
                println!("❌");
                println!("the received message got malformed on the way to us: {err}");
            }
        },
        Ok(None) => {
            println!("❌");
            println!("failed to receive any message back...");
        }
    }

    println!("disconnecting the client before shutting down...");
    client.disconnect().await;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // std::env::set_var(
    //     "RUST_LOG",
    //     "debug,handlebars=warn,tendermint_rpc=warn,h2=warn,hyper=warn,rustls=warn,reqwest=warn,tungstenite=warn,async_tungstenite=warn,tokio_util=warn,tokio_tungstenite=warn,tokio-util=warn",
    // );

    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());
    // setup_tracing_logger();

    args.execute().await
}
