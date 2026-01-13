// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::setup::rebuild_binaries_image;
use std::path::PathBuf;
use tracing::debug;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// Custom tag for the new images
    #[clap(long)]
    custom_tag: Option<String>,

    /// Custom path to root of the monorepo in case this binary has been executed from a different location.
    /// If not provided, it is going to get assumed that the current directory is the monorepo root
    #[clap(long)]
    monorepo_root: Option<PathBuf>,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");

    let orchestrator = LocalnetOrchestrator::new(&args.common).await?;

    orchestrator
        .rebuild_binaries_image(rebuild_binaries_image::Config {
            custom_tag: args.custom_tag,
            monorepo_root: args.monorepo_root,
        })
        .await?;

    Ok(())
}
