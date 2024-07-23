// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::error::NetworkManagerError;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// Path to the `nym-client` binary
    #[clap(long)]
    nym_client_bin: PathBuf,

    #[clap(long)]
    gateway: Option<String>,

    #[clap(long)]
    network_name: Option<String>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> Result<(), NetworkManagerError> {
    let manager = args.common.network_manager().await?;
    let network = manager.load_existing_network(args.network_name).await?;

    let run_cmd = manager
        .init_local_nym_client(args.nym_client_bin, &network, args.gateway)
        .await?;

    if !args.output.is_text() {
        args.output.to_stderr(&run_cmd)
    }

    Ok(())
}
