// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::setup::nym_api;
use crate::orchestrator::state::LocalnetState;
use anyhow::bail;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use tracing::debug;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// Cosmwasm optimizer image used for building and optimising the contracts
    #[clap(long, default_value = "cosmwasm/optimizer:0.17.0")]
    cosmwasm_optimizer_image: String,

    /// Custom path to root of the monorepo in case this binary has been executed from a different location.
    /// If not provided, it is going to get assumed that the current directory is the monorepo root
    #[clap(long)]
    monorepo_root: Option<PathBuf>,

    /// Specify whether the orchestrator can attempt to retrieve previously built cached contracts.
    #[clap(long)]
    allow_cached_build: bool,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");

    let mut orchestrator = LocalnetOrchestrator::new(&args.common).await?;

    if orchestrator.state != LocalnetState::DeployedNymContracts {
        bail!(
            "can't initialise nym api - nym contracts have not already been initialised or nym api is already running. the localnet is in {} state.",
            orchestrator.state
        )
    }

    orchestrator
        .initialise_nym_api(nym_api::Config {
            cosmwasm_optimizer_image: args.cosmwasm_optimizer_image,
            monorepo_root: args.monorepo_root,
            custom_dns: args.common.custom_dns,
            allow_cached_build: args.allow_cached_build,
        })
        .await?;

    Ok(())
}
