// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::setup::cosmwasm_contracts;
use crate::orchestrator::state::LocalnetState;
use anyhow::bail;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;
use tracing::debug;

#[derive(clap::Args, Debug)]
#[clap(group(clap::ArgGroup::new("built-contracts").required(false)))]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// Point to on-disk director containing .wasm files of all required Nym contracts
    #[clap(long, group = "built-contracts")]
    contracts_directory: Option<PathBuf>,

    /// Provide a branch name to be used for attempting to retrieve .wasm files from the ci build server,
    /// e.g. for branch `feature/my-amazing-feature`, the following urls will be used:
    /// - `https://builds.ci.nymte.ch/feature/my-amazing-feature/mixnet_contract.wasm`
    /// - `https://builds.ci.nymte.ch/feature/my-amazing-feature/nym_performance_contract.wasm`
    /// - ...
    /// - etc.
    #[clap(long, group = "built-contracts")]
    ci_build_branch: Option<String>,

    /// Ensure contracts wasm code is fully reproducible by building those them
    /// with linux/amd64 platform and forcing some additional cargo build flags.
    /// Note: it will cause significant (build-time) overhead for M1 Macs
    #[clap(long)]
    reproducible_builds: bool,

    /// Cosmwasm optimizer image used for building and optimising the contracts
    #[clap(long, default_value = "cosmwasm/optimizer:0.17.0")]
    cosmwasm_optimizer_image: String,

    /// Custom path to root of the monorepo in case this binary has been executed from a different location.
    /// If not provided, it is going to get assumed that the current directory is the monorepo root
    #[clap(long)]
    monorepo_root: Option<PathBuf>,

    /// Specify whether the orchestrator can attempt to retrieve previously built cached contracts.
    #[clap(long, conflicts_with = "reproducible_builds")]
    allow_cached_build: bool,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");

    let mut orchestrator = LocalnetOrchestrator::new(&args.common).await?;

    if orchestrator.state != LocalnetState::RunningNyxd {
        bail!(
            "can't initialise cosmwasm contracts - nyxd is not running or the contracts have already been initialised. the localnet is in {} state.",
            orchestrator.state
        )
    }

    orchestrator
        .initialise_contracts(cosmwasm_contracts::Config {
            reproducible_builds: args.reproducible_builds,
            cosmwasm_optimizer_image: args.cosmwasm_optimizer_image,
            explicit_contracts_directory: args.contracts_directory,
            ci_build_branch: args.ci_build_branch,
            monorepo_root: args.monorepo_root,
            allow_cached_build: args.allow_cached_build,
        })
        .await?;

    Ok(())
}
