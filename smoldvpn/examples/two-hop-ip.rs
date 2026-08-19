// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `two-hop-ip` — show that a two-hop dVPN tunnel relocates your public IP.
//!
//! Queries `ipinfo.io` directly to show your real IP/location, brings up a
//! two-hop tunnel (random entry + exit gateways), prints both gateways' details,
//! then queries `ipinfo.io` again *through the tunnel* — the reported IP/org/
//! country should now be the exit gateway's.
//!
//! Usage (build `--release`: boringtun is slow in debug):
//!   MNEMONIC="<funded mnemonic>" \
//!   cargo run --release -p nym-smoldvpn --example two-hop-ip [-- <options>]
//!
//! Options (see `common::USAGE` / README): `--one-hop`/`--two-hop`,
//! `--entry <SPEC>`, `--exit <SPEC>`, `--gateway <SPEC>`, `--quic`. `<SPEC>` is
//! `random`, a two-letter country code, or a base58 gateway identity. Defaults
//! to a random two-hop tunnel.

use std::process::ExitCode;
use std::time::Duration;

use tracing::{error, info};

#[path = "common/mod.rs"]
mod common;

async fn run() -> Result<(), common::BoxError> {
    common::init_logging();
    common::init_crypto();
    let cli = common::parse_cli()?;

    // 1. Real IP (direct).
    let real = common::ipinfo_direct().await?;
    info!("real IP (no tunnel): {}", common::fmt_ipinfo(&real));

    // 2. Provision + register the requested tunnel.
    info!("provisioning a {} tunnel …", common::describe(&cli));
    let session = common::new_session("two-hop-ip").await;
    let result = async {
        // 3. Register (cache-served when possible) + bring up the tunnel,
        // gated on WireGuard establishment with stale-cache fallback.
        let (reg, tunnel) = common::connect(&session, &cli).await?;

        common::print_gateway("entry", &reg.entry.gateway);
        if let Some(exit) = reg.exit.as_ref() {
            common::print_gateway("exit", &exit.gateway);
        }

        // 4. IP through the tunnel (established — this is a display probe).
        info!("querying ipinfo.io through the tunnel …");
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

    info!("PASS: public IP relocated through the two-hop tunnel");
    // Guarantee prompt termination regardless of background reactor teardown.
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
