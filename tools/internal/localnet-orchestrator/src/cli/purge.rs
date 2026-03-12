// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::setup::purge;
use clap::ArgAction;
use std::path::PathBuf;
use tracing::debug;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// Remove any built docker and container images
    #[clap(long, action = ArgAction::Set, default_value_t = true)]
    remove_images: bool,

    /// Remove any cached build data
    #[clap(long, action = ArgAction::Set, default_value_t = true)]
    remove_cache: bool,

    /// Custom path to root of the monorepo in case this binary has been executed from a different location.
    /// If not provided, it is going to get assumed that the current directory is the monorepo root
    #[clap(long)]
    monorepo_root: Option<PathBuf>,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");

    LocalnetOrchestrator::new(&args.common)
        .await?
        .purge_localnet(purge::Config {
            remove_images: args.remove_images,
            remove_cache: args.remove_cache,
            monorepo_root: args.monorepo_root,
        })
        .await
}
