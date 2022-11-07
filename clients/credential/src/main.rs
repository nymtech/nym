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
        use network_defaults::setup_env;
        use clap::CommandFactory;
        use completions::fig_generate;

        use clap::Parser;
        use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};

        #[derive(Parser)]
        #[clap(author = "Nymtech", version, about)]
        struct Cli {
            /// Path pointing to an env file that configures the client.
            #[clap(long)]
            pub(crate) config_env_file: Option<std::path::PathBuf>,

            /// Path where the sqlite credental database will be located.
            /// It should point to a $HOME/$CLIENT_ID/data/db.sqlite file of
            /// the client that is supposed to use the credential.
            #[clap(long)]
            pub(crate) credential_db_path: std::path::PathBuf,

            #[clap(subcommand)]
            command: Commands,
        }

        #[tokio::main]
        async fn main() -> Result<()> {
            let args = Cli::parse();
            setup_env(args.config_env_file.clone());

            let shared_storage = credential_storage::initialise_storage(args.credential_db_path.clone()).await;
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

            let bin_name = "nym-credential-client";

            match &args.command {
                Commands::Deposit(m) => m.execute(&mut db, shared_storage).await?,
                Commands::ListDeposits(m) => m.execute(&mut db, shared_storage).await?,
                Commands::GetCredential(m) => m.execute(&mut db, shared_storage).await?,
                Commands::Completions(s) => s.generate(&mut crate::Cli::into_app(), bin_name),
                Commands::GenerateFigSpec => fig_generate(&mut crate::Cli::into_app(), bin_name)
            }

            Ok(())
        }
    } else {
        fn main() {
            println!("Crate only designed for coconut feature");
        }
    }
}
