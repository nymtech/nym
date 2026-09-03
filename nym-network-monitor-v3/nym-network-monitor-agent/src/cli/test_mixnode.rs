// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::wave::MixnetWave;
use crate::cli::manual::ManualTargetArgs;
use nym_network_monitor_orchestrator_requests::models::TestKind;
use nym_task::ShutdownToken;
use tracing::info;

/// Arguments for the `test-mixnode-stress` and `test-mixnode-liveness` subcommands.
///
/// One argument set for both, because the two differ only in the profile applied: the packets, the
/// route and the keys are identical, which is the property the shared [`MixnetWave`] exists to keep.
#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    target: ManualTargetArgs,
}

/// Runs a one-shot probe of the specified node under `kind` and logs the result.
pub(crate) async fn execute(args: Args, kind: TestKind) -> anyhow::Result<()> {
    let keys = args.target.load_keys()?;

    let node = args.target.discover().await?.require_mixnode()?;

    info!("running a one-shot {kind} probe of {}", node.address);

    // a wave of exactly one, which is the same path a stress assignment takes from the orchestrator
    MixnetWave::new(
        args.target.build_tester_config()?,
        kind,
        keys.client_address(),
        keys.noise_key.clone(),
        vec![node],
        // a manual run IS the whole process, so its wave is the root
        ShutdownToken::new(),
    )?
    .run(|report| async move {
        // nothing to submit to from here, so the report is simply logged
        info!("{:#?}", report.result);
        Ok(())
    })
    .await
}
