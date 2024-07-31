// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::CommonArgs;
use crate::error::NetworkManagerError;
use crate::helpers::default_storage_dir;
use crate::manager::env::Env;
use crate::manager::network::LoadedNetwork;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// Path containing .wasm files of all contracts
    #[clap(long)]
    built_contracts: PathBuf,

    #[clap(long)]
    network_name: Option<String>,

    #[clap(long)]
    signer_data_output_directory: Option<PathBuf>,

    /// The URLs of that the DKG parties would have put in the contract
    #[clap(long, value_delimiter = ',')]
    api_endpoints: Vec<Url>,

    /// Path to the contract built from the `dkg-bypass-contract` directory
    #[clap(long)]
    bypass_dkg_contract: PathBuf,

    /// Specifies custom duration of mixnet epochs
    /// It's recommended to set it to rather low value (like 60s) if you intend to bond the mixnet afterward.
    #[clap(long)]
    custom_epoch_duration_secs: Option<u64>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> Result<(), NetworkManagerError> {
    let manager = args.common.network_manager().await?;

    let network: LoadedNetwork = manager
        .initialise_new_network(
            args.built_contracts,
            args.network_name,
            args.custom_epoch_duration_secs.map(Duration::from_secs),
        )
        .await?
        .into();

    let signer_data_output_directory = if let Some(explicit) = args.signer_data_output_directory {
        explicit
    } else {
        default_storage_dir().join(&network.name)
    };

    let env = Env::from(&network);

    manager
        .attempt_bypass_dkg(
            args.api_endpoints,
            &network,
            args.bypass_dkg_contract,
            signer_data_output_directory,
        )
        .await?;

    println!("add the following to your .env file: \n{env}",);

    Ok(())
}
