// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NetworkManagerError;
use crate::helpers::default_db_file;
use crate::manager::NetworkManager;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Path containing .wasm files of all contracts
    #[clap(long)]
    built_contracts: PathBuf,

    #[clap(long)]
    master_mnemonic: Option<bip39::Mnemonic>,

    #[clap(long)]
    rpc_endpoint: Option<Url>,

    #[clap(long)]
    storage_path: Option<PathBuf>,

    #[clap(long)]
    network_name: Option<String>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> Result<(), NetworkManagerError> {
    let storage = args.storage_path.unwrap_or_else(default_db_file);

    let network = NetworkManager::new(storage, args.master_mnemonic, args.rpc_endpoint)
        .await?
        .initialise_new_network(args.built_contracts, args.network_name)
        .await?;

    println!(
        "add the following to your .env file: \n{}",
        network.unchecked_to_env_file_section()
    );

    Ok(())
}
