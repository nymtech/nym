// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::CommonArgs;
use crate::error::NetworkManagerError;
use crate::manager::env::Env;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use std::time::Duration;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// Path containing .wasm files of all contracts
    #[clap(long)]
    built_contracts: PathBuf,

    #[clap(long)]
    network_name: Option<String>,

    /// Specifies custom duration of mixnet epochs
    /// It's recommended to set it to rather low value (like 60s) if you intend to bond the mixnet afterward.
    #[clap(long)]
    custom_epoch_duration_secs: Option<u64>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> Result<(), NetworkManagerError> {
    let network = args
        .common
        .network_manager()
        .await?
        .initialise_new_network(
            args.built_contracts,
            args.network_name,
            args.custom_epoch_duration_secs.map(Duration::from_secs),
        )
        .await?
        .into_loaded();

    let env = Env::from(&network);
    println!("add the following to your .env file: \n{env}",);

    Ok(())
}
