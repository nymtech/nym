// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `two-hop-quic` — a two-hop dVPN tunnel whose ENTRY leg is carried over a QUIC
//! bridge (for clients blocked from plain WireGuard/UDP).
//!
//! Shows your real IP via `ipinfo.io`, brings up a two-hop tunnel whose entry
//! gateway MUST be QUIC-bridge-capable (selected from the dVPN directory; the run
//! fails with `NoQuicGateway` if none is available), prints both gateways, then
//! queries `ipinfo.io` through the tunnel to show the IP/org/country moved to the
//! exit gateway. QUIC only ever fronts the two-hop entry leg.
//!
//! Usage (build `--release`: boringtun is slow in debug):
//!   MNEMONIC="<funded mnemonic>" \
//!   cargo run --release -p nym-smoldvpn --example two-hop-quic
//!
//! The QUIC gateway set comes from the dVPN directory (see
//! `common::DEFAULT_DVPN_DIRECTORY`; override with `DVPN_DIRECTORY_URL`).

use std::process::ExitCode;
use std::time::Duration;

use tracing::{error, info};

#[path = "common/mod.rs"]
mod common;

async fn run() -> Result<(), common::BoxError> {
    common::init_logging();
    common::init_crypto();
    // QUIC + two-hop by definition; still honour --entry/--exit/--gateway selectors.
    let mut cli = common::parse_cli()?;
    if !cli.two_hop {
        return Err("two-hop-quic is two-hop only (QUIC fronts the entry leg)".into());
    }
    cli.quic = true;

    // 1. Real IP (direct).
    let real = common::ipinfo_direct().await?;
    info!("real IP (no tunnel): {}", common::fmt_ipinfo(&real));

    // 2. Register a two-hop tunnel, requiring a QUIC-capable entry gateway.
    info!("provisioning a two-hop tunnel with a QUIC entry gateway …");
    let session = common::new_session("two-hop-quic").await;
    let result = async {
        // 3. Register (cache-served when possible) + bring up the QUIC-fronted
        // tunnel, gated on WireGuard establishment with stale-cache fallback.
        let (reg, tunnel) = common::connect(&session, &cli).await?;

        common::print_gateway("entry (QUIC)", &reg.entry.gateway);
        common::print_gateway(
            "exit",
            &reg.exit.as_ref().expect("two-hop has an exit hop").gateway,
        );
        if let Some(bridge) = &reg.entry.bridge {
            info!(
                "entry QUIC bridge: {} addr(s), SNI {}",
                bridge.addresses.len(),
                bridge.sni_host.as_deref().unwrap_or("(none)")
            );
        }

        // 4. IP through the tunnel (established — this is a display probe).
        info!("querying ipinfo.io through the QUIC-fronted tunnel …");
        let via = common::ipinfo_display(&tunnel).await?;
        info!("IP through the tunnel: {}", common::fmt_ipinfo(&via));
        info!("(the tunnelled IP/org/country should be the EXIT gateway's)");

        // 5. Tear down (bounded — live multi-threaded teardown can be slow).
        let _ = tokio::time::timeout(Duration::from_secs(5), tunnel.shutdown()).await;
        Ok::<(), common::BoxError>(())
    }
    .await;

    // Close the session's credential store cleanly (checkpoints the sqlite WAL;
    // stored tickets are retained) whether the flow succeeded or failed.
    session.shutdown().await;
    result?;

    info!("PASS: public IP relocated through the QUIC-fronted two-hop tunnel");
    std::process::exit(0);
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> ExitCode {
    if let Err(e) = run().await {
        error!("{e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
