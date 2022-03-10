// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod commands;

use commands::{Commands, Execute};

use clap::Parser;

#[derive(Parser)]
#[clap(author = "Nymtech", version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    match &args.command {
        Commands::Deposit(m) => m.execute().await,
        Commands::GetCredential(m) => m.execute().await,
    }
}
