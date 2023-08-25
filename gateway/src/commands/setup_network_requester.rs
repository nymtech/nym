// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::helpers::initialise_local_network_requester;
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct CmdArgs {
    /// Path to custom location for network requester's config.
    #[clap(long)]
    custom_config_path: Option<PathBuf>,
}

pub async fn execute(args: CmdArgs) -> anyhow::Result<()> {
    initialise_local_network_requester()?;

    Ok(())
}
