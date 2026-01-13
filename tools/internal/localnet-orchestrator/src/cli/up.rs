// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::setup::{cosmwasm_contracts, nym_api, nym_nodes, nyxd, up};
use crate::orchestrator::state::LocalnetState;
use anyhow::bail;
use std::path::PathBuf;
use tracing::debug;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    #[clap(long, default_value = "https://github.com/nymtech/nyxd.git")]
    nyxd_repo: Url,

    /// Absolute path (from the repo root) to the location of the Dockerfile used for building nyxd
    #[clap(long, default_value = "Dockerfile.dev")]
    nyxd_dockerfile_path: String,

    #[clap(long, default_value = "v0.60.1")]
    nyxd_tag: String,

    /// Cosmwasm optimizer image used for building and optimising the contracts
    #[clap(long, default_value = "cosmwasm/optimizer:0.17.0")]
    cosmwasm_optimizer_image: String,

    /// Custom path to root of the monorepo in case this binary has been executed from a different location.
    /// If not provided, it is going to get assumed that the current directory is the monorepo root
    #[clap(long)]
    monorepo_root: Option<PathBuf>,

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

    /// Specify whether the orchestrator can attempt to retrieve previously built cached contracts.
    #[clap(long, conflicts_with = "reproducible_builds")]
    allow_cached_build: bool,

    /// Specify whether internal service providers should run in open proxy mode
    #[clap(long)]
    open_proxy: bool,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");

    let mut orchestrator = LocalnetOrchestrator::new(&args.common).await?;

    // TODO: allow non-fresh state
    if orchestrator.state != LocalnetState::Uninitialised {
        bail!("orchestrator is not in a fresh state")
    }

    orchestrator
        .start_localnet(up::Config {
            nyxd_setup: nyxd::Config {
                nyxd_repo: args.nyxd_repo,
                nyxd_dockerfile_path: args.nyxd_dockerfile_path,
                custom_dns: args.common.custom_dns.clone(),
                nyxd_tag: args.nyxd_tag,
            },
            contracts_setup: cosmwasm_contracts::Config {
                reproducible_builds: args.reproducible_builds,
                cosmwasm_optimizer_image: args.cosmwasm_optimizer_image.clone(),
                explicit_contracts_directory: args.contracts_directory,
                ci_build_branch: args.ci_build_branch,
                monorepo_root: args.monorepo_root.clone(),
                allow_cached_build: args.allow_cached_build,
            },
            nym_api_setup: nym_api::Config {
                cosmwasm_optimizer_image: args.cosmwasm_optimizer_image,
                monorepo_root: args.monorepo_root.clone(),
                custom_dns: args.common.custom_dns.clone(),
                allow_cached_build: args.allow_cached_build,
            },
            nym_nodes_setup: nym_nodes::Config {
                monorepo_root: args.monorepo_root,
                custom_dns: args.common.custom_dns,
                open_proxy: args.open_proxy,
            },
        })
        .await
}
