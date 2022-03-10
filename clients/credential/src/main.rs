// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod commands;

use commands::{Commands, Execute};

use clap::Parser;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};

#[derive(Parser)]
#[clap(author = "Nymtech", version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() {
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
        Commands::Deposit(m) => m.execute(&mut db).await,
        Commands::GetCredential(m) => m.execute(&mut db).await,
    }
}
