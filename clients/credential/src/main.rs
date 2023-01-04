// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

cfg_if::cfg_if! {
    if #[cfg(feature = "coconut")] {

        mod client;
        mod commands;
        mod error;
        mod state;

        use error::Result;
        use network_defaults::setup_env;
        use completions::fig_generate;
        use commands::*;
        use config::{DATA_DIR, DB_FILE_NAME};

        use clap::{CommandFactory, Parser};

        #[derive(Parser)]
        #[clap(author = "Nymtech", version, about)]
        struct Cli {
            /// Path pointing to an env file that configures the client.
            #[clap(short, long)]
            pub(crate) config_env_file: Option<std::path::PathBuf>,

            #[clap(subcommand)]
            pub(crate) command: Command,
        }

        #[tokio::main]
        async fn main() -> Result<()> {
            let args = Cli::parse();
            setup_env(args.config_env_file.clone());
            let bin_name = "nym-credential-client";

            match args.command {
                Command::Run(r) => {
                    let db_path = r.client_home_directory.join(DATA_DIR).join(DB_FILE_NAME);
                    let shared_storage = credential_storage::initialise_storage(db_path).await;

                    let state = deposit(&r.nymd_url, &r.mnemonic, r.amount).await?;
                    get_credential(&state, shared_storage).await?;
                }
                Command::Completions(c) => c.generate(&mut crate::Cli::command(), bin_name),
                Command::GenerateFigSpec => fig_generate(&mut crate::Cli::command(), bin_name)
            }

            Ok(())
        }
    } else {
        fn main() {
            println!("Crate only designed for coconut feature");
        }
    }
}
