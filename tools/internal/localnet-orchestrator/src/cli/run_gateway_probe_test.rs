// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::state::LocalnetState;
use anyhow::bail;
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

    /// additional, optional flags to pass when starting the gateway probe
    /// e.g. "--ignore-egress-epoch-role --netstack-args='...'"
    #[clap(long)]
    probe_args: Option<String>,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");

    let orchestrator = LocalnetOrchestrator::new(&args.common).await?;
    if orchestrator.state != LocalnetState::RunningNymNodes {
        bail!(
            "can't test the gateway probe as the localnet does not appear to be running. the localnet is in {} state.",
            orchestrator.state
        )
    }

    orchestrator
        .run_gateway_probe(args.monorepo_root, args.probe_args)
        .await?;

    Ok(())
}
