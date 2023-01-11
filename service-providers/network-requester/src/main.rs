// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::CommandFactory;
use clap::Subcommand;
use clap::{Args, Parser};
use completions::{fig_generate, ArgShell};
use logging::setup_logging;

use error::NetworkRequesterError;
use nymsphinx::addressing::clients::Recipient;

mod allowed_hosts;
mod core;
mod error;
mod reply;
mod socks5;
mod statistics;
mod websocket;

const ENABLE_STATISTICS: &str = "enable-statistics";

#[derive(Args)]
struct Run {
    /// Specifies whether this network requester should run in 'open-proxy' mode
    #[clap(long)]
    open_proxy: bool,

    /// Websocket port to bind to. Defaults to `network_defaults::DEFAULT_WEBSOCKET_LISTENING_PORT` (currently 1977)
    #[clap(long)]
    websocket_port: Option<String>,

    /// Enable service anonymized statistics that get sent to a statistics aggregator server
    #[clap(long)]
    enable_statistics: bool,

    /// Mixnet client address where a statistics aggregator is running. The default value is a Nym aggregator client
    #[clap(long)]
    statistics_recipient: Option<String>,
}

impl Run {
    async fn execute(&self) -> Result<(), NetworkRequesterError> {
        if self.open_proxy {
            println!("\n\nYOU HAVE STARTED IN 'OPEN PROXY' MODE. ANYONE WITH YOUR CLIENT ADDRESS CAN MAKE REQUESTS FROM YOUR MACHINE. PLEASE QUIT IF YOU DON'T UNDERSTAND WHAT YOU'RE DOING.\n\n");
        }

        if self.enable_statistics {
            println!("\n\nTHE NETWORK REQUESTER STATISTICS ARE ENABLED. IT WILL COLLECT AND SEND ANONYMIZED STATISTICS TO A CENTRAL SERVER. PLEASE QUIT IF YOU DON'T WANT THIS TO HAPPEN AND START WITHOUT THE {} FLAG .\n\n", ENABLE_STATISTICS);
        }

        let stats_provider_addr = self
            .statistics_recipient
            .as_ref()
            .map(Recipient::try_from_base58_string)
            .transpose()
            .unwrap_or(None);

        let websocket_address = format!(
            "ws://localhost:{}",
            self.websocket_port
                .as_ref()
                .unwrap_or(&network_defaults::DEFAULT_WEBSOCKET_LISTENING_PORT.to_string())
        );

        log::info!("Starting socks5 service provider");
        let mut server = core::ServiceProvider::new(
            websocket_address,
            self.open_proxy,
            self.enable_statistics,
            stats_provider_addr,
        );
        server.run().await
    }
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Run network requester
    Run(Run),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

pub(crate) async fn execute(args: Cli) -> Result<(), NetworkRequesterError> {
    let bin_name = "nym-network-requester";

    match &args.command {
        Commands::Run(r) => r.execute().await?,
        Commands::Completions(s) => s.generate(&mut crate::Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut crate::Cli::command(), bin_name),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), NetworkRequesterError> {
    setup_logging();
    let args = Cli::parse();

    execute(args).await
}
