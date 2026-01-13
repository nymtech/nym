// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CommonArgs;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::setup::nyxd;
use crate::orchestrator::state::LocalnetState;
use anyhow::bail;
use nym_bin_common::output_format::OutputFormat;
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

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");

    let mut orchestrator = LocalnetOrchestrator::new(&args.common).await?;
    if orchestrator.state != LocalnetState::Uninitialised {
        bail!(
            "can't initialise nyxd as it appears to have already been initialised. the localnet is in {} state.",
            orchestrator.state
        )
    }

    orchestrator
        .initialise_nyxd(nyxd::Config {
            nyxd_repo: args.nyxd_repo,
            nyxd_dockerfile_path: args.nyxd_dockerfile_path,
            custom_dns: args.common.custom_dns,
            nyxd_tag: args.nyxd_tag,
        })
        .await?;

    Ok(())
}
