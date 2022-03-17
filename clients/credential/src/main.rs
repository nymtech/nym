// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod client;
mod commands;
mod error;
mod state;

use commands::{Commands, Execute};
use error::Result;

use clap::Parser;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};

pub const MNEMONIC: &str = "sun surge soon stomach flavor country gorilla dress oblige stamp attract hip soldier agree steel prize nuclear know enjoy arm bargain always theme matter";
pub const NYMD_URL: &str = "http://127.0.0.1:26657";
pub const CONTRACT_ADDRESS: &str = "nymt1w27ekqvvtzfanfxnkw4jx2f8gdfeqwd3qxku6y";
pub const SIGNER_AUTHORITIES: [&str; 3] = [
    "http://127.0.0.1:8080",
    "http://127.0.0.1:8081",
    "http://127.0.0.1:8082",
];

#[derive(Parser)]
#[clap(author = "Nymtech", version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let mut db = match PickleDb::load(
        "credential.db",
        PickleDbDumpPolicy::AutoDump,
        SerializationMethod::Json,
    ) {
        Ok(db) => db,
        Err(_) => PickleDb::new(
            "credential.db",
            PickleDbDumpPolicy::AutoDump,
            SerializationMethod::Json,
        ),
    };

    match &args.command {
        Commands::Deposit(m) => m.execute(&mut db).await?,
        Commands::ListDeposits(m) => m.execute(&mut db).await?,
        Commands::GetCredential(m) => m.execute(&mut db).await?,
    }

    Ok(())
}
