// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::gateway::wave::GatewayLivenessWave;
use crate::cli::manual::ManualTargetArgs;
use nym_task::ShutdownToken;
use tracing::info;

/// Arguments for the `test-gateway-liveness` subcommand.
///
/// Named for the kind as well as the role, and kept in its own module rather than folded in beside
/// the mixnode commands, because liveness is the ONLY kind a gateway is probed for: a gateway stress
/// test would not resemble this one, and nothing here would be reusable if one were ever added.
#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    target: ManualTargetArgs,
}

/// Runs a one-shot gateway liveness probe of the specified node and logs both phases.
pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let keys = args.target.load_keys()?;
    let gateway = args.target.discover().await?.require_gateway()?;

    info!(
        "running a one-shot gateway liveness probe of {}, whose client websocket is on port {}",
        gateway.mixnet.address, gateway.clients_ws_port
    );

    GatewayLivenessWave::new(
        args.target.build_tester_config()?,
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
