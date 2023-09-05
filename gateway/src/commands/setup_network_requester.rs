// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::helpers::{initialise_local_network_requester, try_load_current_config};
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct CmdArgs {
    /// The id of the gateway you want to initialise local network requester for.
    #[clap(long)]
    id: String,

    /// Path to custom location for network requester's config.
    #[clap(long)]
    custom_config_path: Option<PathBuf>,
}

pub async fn execute(args: CmdArgs) -> anyhow::Result<()> {
    let config = try_load_current_config(&args.id)?;

    todo!()
    // initialise_local_network_requester(&config).await?;

    // Ok(())
}
