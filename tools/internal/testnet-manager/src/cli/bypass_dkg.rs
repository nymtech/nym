// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::CommonArgs;
use crate::error::NetworkManagerError;
use crate::helpers::default_storage_dir;
use std::path::PathBuf;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    #[clap(long)]
    signer_data_output_directory: Option<PathBuf>,

    #[clap(long)]
    network_name: Option<String>,

    /// The URLs of that the DKG parties would have put in the contract
    #[clap(long, value_delimiter = ',')]
    api_endpoints: Vec<Url>,

    /// Path to the contract built from the `dkg-bypass-contract` directory
    #[clap(long)]
    bypass_dkg_contract: PathBuf,
}

pub(crate) async fn execute(args: Args) -> Result<(), NetworkManagerError> {
    let manager = args.common.network_manager().await?;
    let network = manager.load_existing_network(args.network_name).await?;

    let signer_data_output_directory = if let Some(explicit) = args.signer_data_output_directory {
        explicit
    } else {
        default_storage_dir().join(&network.name)
    };

    manager
        .attempt_bypass_dkg(
            args.api_endpoints,
            &network,
            args.bypass_dkg_contract,
            signer_data_output_directory,
        )
        .await?;

    Ok(())
}
