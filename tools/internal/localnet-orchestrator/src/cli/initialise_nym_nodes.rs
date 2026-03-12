// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::setup::nym_nodes;
use crate::orchestrator::state::LocalnetState;
use anyhow::bail;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use tracing::debug;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// Custom path to root of the monorepo in case this binary has been executed from a different location.
    /// If not provided, it is going to get assumed that the current directory is the monorepo root
    #[clap(long)]
    monorepo_root: Option<PathBuf>,

    /// Specify whether internal service providers should run in open proxy mode
    #[clap(long)]
    open_proxy: bool,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");

    let mut orchestrator = LocalnetOrchestrator::new(&args.common).await?;

    if orchestrator.state != LocalnetState::RunningNymApi {
        bail!(
            "can't initialise nym nodes - nym api has not already been initialised or nym nodes are already running. the localnet is in {} state.",
            orchestrator.state
        )
    }

    orchestrator
        .initialise_nym_nodes(nym_nodes::Config {
            monorepo_root: args.monorepo_root,
            custom_dns: args.common.custom_dns,
            open_proxy: args.open_proxy,
        })
        .await?;

    Ok(())
}
