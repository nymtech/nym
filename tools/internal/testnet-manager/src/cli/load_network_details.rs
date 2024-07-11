// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::helpers::default_db_file;
use crate::manager::env::Env;
use crate::manager::NetworkManager;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(long)]
    network_name: Option<String>,

    #[clap(long)]
    storage_path: Option<PathBuf>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> Result<(), NetworkManagerError> {
    let storage = args.storage_path.unwrap_or_else(default_db_file);

    let network = NetworkManager::new(storage, None, None)
        .await?
        .load_existing_network(args.network_name)
        .await?;

    let env = Env::from(&network);
    println!("add the following to your .env file: \n{env}",);

    Ok(())
}
