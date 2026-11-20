// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::gateway::wave::GatewayWave;
use crate::cli::manual::ManualTargetArgs;
use nym_network_monitor_orchestrator_requests::models::TestKind;
use nym_task::ShutdownToken;
use tracing::info;

/// Arguments for the `test-gateway-liveness` and `test-gateway-stress` subcommands.
///
/// One argument set for both, because the two differ only in the profile applied: the session, the
/// two phases and the keys are identical. Kept apart from the mixnode commands because a gateway is
/// probed by a wave of its own, over a client session rather than a Noise connection.
#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    target: ManualTargetArgs,
}

/// Runs a one-shot probe of the specified gateway under `kind` and logs both phases.
pub(crate) async fn execute(args: Args, kind: TestKind) -> anyhow::Result<()> {
    let keys = args.target.load_keys()?;
    let gateway = args.target.discover().await?.require_gateway()?;

    info!(
        "running a one-shot {kind} probe of {}, whose client websocket is on port {}",
        gateway.mixnet.address, gateway.clients_ws_port
    );

    // a wave of exactly one, which is the same path a gateway stress assignment takes from the
    // orchestrator
    GatewayWave::new(
        args.target.build_tester_config()?,
        kind,
        keys.client_identity.clone(),
        keys.noise_key.clone(),
        vec![gateway],
        // a manual run IS the whole process, so its wave is the root
        ShutdownToken::new(),
    )?
    .run(|report| async move {
        // nothing to submit to from here, so both phases are simply logged
        info!("{:#?}", report.result);
        Ok(())
    })
    .await
}
