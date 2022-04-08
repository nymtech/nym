// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

cfg_if::cfg_if! {
    if #[cfg(feature = "coconut")] {

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
        pub const CONTRACT_ADDRESS: &str = "nymt1vhjnzk9ly03dugffvzfcwgry4dgc8x0sscmfl2";
        pub const SIGNER_AUTHORITIES: [&str; 1] = [
            "http://127.0.0.1:8080",
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

            let shared_storage = credential_storage::initialise_storage(std::path::PathBuf::from("/tmp/credential.db")).await;
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
                Commands::Deposit(m) => m.execute(&mut db, shared_storage).await?,
                Commands::ListDeposits(m) => m.execute(&mut db, shared_storage).await?,
                Commands::GetCredential(m) => m.execute(&mut db, shared_storage).await?,
            }

            Ok(())
        }
    } else {
        fn main() {
            println!("Crate only designed for coconut feature");
        }
    }
}
