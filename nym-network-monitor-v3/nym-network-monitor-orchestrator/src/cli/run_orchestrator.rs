// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use tracing::info;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Bearer token required by the agents requesting work assignments
    /// and submitting the results
    #[clap(long, env = NYM_NETWORK_MONITOR_ORCHESTRATOR_TOKEN_ARG)]
    orchestrator_token: String,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    info!("Starting network monitor orchestrator");
    Ok(())
}
