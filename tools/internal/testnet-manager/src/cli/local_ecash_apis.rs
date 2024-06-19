// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::helpers::default_db_file;
use crate::manager::network::LoadedNetwork;
use crate::manager::NetworkManager;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use tempfile::tempdir;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Path to the `nym-api` binary
    #[clap(long)]
    nym_api_bin: PathBuf,

    /// Path containing .wasm files of all contracts
    #[clap(long)]
    built_contracts: PathBuf,

    #[clap(long)]
    number_of_apis: usize,

    #[clap(long)]
    master_mnemonic: Option<bip39::Mnemonic>,

    #[clap(long)]
    rpc_endpoint: Option<Url>,

    #[clap(long)]
    storage_path: Option<PathBuf>,

    #[clap(long)]
    network_name: Option<String>,

    /// Path to the contract built from the `dkg-bypass-contract` directory
    #[clap(long)]
    bypass_dkg_contract: PathBuf,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> Result<(), NetworkManagerError> {
    let endpoints = (0..args.number_of_apis)
        .map(|i| format!("http://127.0.0.1:{}", 10000 + i).parse().unwrap())
        .collect::<Vec<Url>>();

    let storage = args.storage_path.unwrap_or_else(default_db_file);

    let manager = NetworkManager::new(storage, args.master_mnemonic, args.rpc_endpoint).await?;

    let network: LoadedNetwork = manager
        .initialise_new_network(args.built_contracts, args.network_name)
        .await?
        .into();

    let temp_output = tempdir()?;

    let signer_details = manager
        .attempt_bypass_dkg(
            endpoints,
            &network,
            args.bypass_dkg_contract,
            temp_output.path(),
        )
        .await?;

    let run_cmds = manager
        .setup_local_apis(args.nym_api_bin, &network, signer_details)
        .await?;

    if !args.output.is_text() {
        args.output.to_stderr(&run_cmds)
    }

    Ok(())
}
