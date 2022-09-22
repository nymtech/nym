// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::CommandFactory;
use clap::Subcommand;
use clap::{Args, Parser};
use completions::{fig_generate, ArgShell};

use network_defaults::DEFAULT_WEBSOCKET_LISTENING_PORT;
use nymsphinx::addressing::clients::Recipient;

mod allowed_hosts;
mod connection;
mod core;
mod statistics;
mod websocket;

const ENABLE_STATISTICS: &str = "enable-statistics";

#[derive(Args)]
struct Run {
    /// Specifies whether this network requester should run in 'open-proxy' mode
    open_proxy: bool,

    /// Websocket port to bind to
    websocket_port: Option<String>,

    /// Enable service anonymized statistics that get sent to a statistics aggregator server
    enable_statistics: bool,

    /// Mixnet client address where a statistics aggregator is running. The default value is a Nym aggregator client
    statistics_recipient: Option<String>,
}

impl Run {
    async fn execute(&self) {
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

        let uri = format!(
            "ws://localhost:{}",
            self.websocket_port
                .as_ref()
                .unwrap_or(&DEFAULT_WEBSOCKET_LISTENING_PORT.to_string())
        );

        println!("Starting socks5 service provider:");
        let mut server = core::ServiceProvider::new(
            uri,
            self.open_proxy,
            self.enable_statistics,
            stats_provider_addr,
        );
        server.run().await;
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

pub(crate) async fn execute(args: Cli) {
    let bin_name = "nym-network-requester";

    match &args.command {
        Commands::Run(r) => r.execute().await,
        Commands::Completions(s) => s.generate(&mut crate::Cli::into_app(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut crate::Cli::into_app(), bin_name),
    }
}

#[tokio::main]
async fn main() {
    setup_logging();
    let args = Cli::parse();

    execute(args).await;
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .init();
}
