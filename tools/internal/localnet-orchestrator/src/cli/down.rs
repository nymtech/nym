// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::orchestrator::LocalnetOrchestrator;
use tracing::debug;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");

    LocalnetOrchestrator::new(&args.common)
        .await?
        .stop_localnet()
        .await
}
