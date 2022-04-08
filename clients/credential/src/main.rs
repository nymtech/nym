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

        pub const MNEMONIC: &str = "jazz fatigue diagram account outer wrist slide cherry mother grid network pause wolf pig round answer mail junior better hair dismiss toward access end";
        pub const NYMD_URL: &str = "http://127.0.0.1:26657";
        pub const CONTRACT_ADDRESS: &str = "nymt1nc5tatafv6eyq7llkr2gv50ff9e22mnfp9pc5s";
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
