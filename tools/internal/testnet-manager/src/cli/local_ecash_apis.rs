// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::error::NetworkManagerError;
use crate::manager::network::LoadedNetwork;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::tempdir;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// Path to the `nym-api` binary
    #[clap(long)]
    nym_api_bin: PathBuf,

    /// Path containing .wasm files of all contracts
    #[clap(long)]
    built_contracts: PathBuf,

    #[clap(long)]
    number_of_apis: usize,

    #[clap(long)]
    network_name: Option<String>,

    /// Path to the contract built from the `dkg-bypass-contract` directory
    #[clap(long)]
    bypass_dkg_contract: PathBuf,

    /// Specifies custom duration of mixnet epochs
    #[clap(long)]
    custom_epoch_duration_secs: Option<u64>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> Result<(), NetworkManagerError> {
    let endpoints = (0..args.number_of_apis)
        .map(|i| format!("http://127.0.0.1:{}", 10000 + i).parse().unwrap())
        .collect::<Vec<Url>>();

    let manager = args.common.network_manager().await?;

    let network: LoadedNetwork = manager
        .initialise_new_network(
            args.built_contracts,
            args.network_name,
            args.custom_epoch_duration_secs.map(Duration::from_secs),
        )
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
