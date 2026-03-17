// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::common::CommonArgs;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common_args: CommonArgs,

    /// Address of the orchestrator for requesting work assignments
    #[clap(long)]
    orchestrator_address: Url,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    todo!()
}
