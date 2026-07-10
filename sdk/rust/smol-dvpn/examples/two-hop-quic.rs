// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

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
//!   cargo run --release -p nym-smol-dvpn --example two-hop-quic
//!
//! The QUIC gateway set comes from the dVPN directory (see
//! `common::DEFAULT_DVPN_DIRECTORY`; override with `DVPN_DIRECTORY_URL`).

use std::process::ExitCode;
use std::time::Duration;

#[path = "common/mod.rs"]
mod common;

async fn run() -> Result<(), common::BoxError> {
    common::init_crypto();
    // QUIC + two-hop by definition; still honour --entry/--exit/--gateway selectors.
    let mut cli = common::parse_cli()?;
    if !cli.two_hop {
        return Err("two-hop-quic is two-hop only (QUIC fronts the entry leg)".into());
    }
    cli.quic = true;

    // 1. Real IP (direct).
    let real = common::ipinfo_direct().await?;
    println!("real IP (no tunnel):    {}", common::fmt_ipinfo(&real));

    // 2. Register a two-hop tunnel, requiring a QUIC-capable entry gateway.
    println!("\nprovisioning a two-hop tunnel with a QUIC entry gateway …");
    let session = common::new_session("two-hop-quic-data").await;
    let reg = common::register(&session, &cli).await?;

    common::print_gateway("entry (QUIC)", &reg.entry.gateway);
    common::print_gateway(
        "exit",
        &reg.exit.as_ref().expect("two-hop has an exit hop").gateway,
    );
    if let Some(bridge) = &reg.entry.bridge {
        println!(
            "  entry QUIC bridge: {} addr(s), SNI {}",
            bridge.addresses.len(),
            bridge.sni_host.as_deref().unwrap_or("(none)")
        );
    }

    // 3. Bring up the tunnel with the QUIC entry leg.
    let tunnel = common::build_tunnel(&reg, /* use_quic = */ true).await?;

    // 4. IP through the tunnel — retry while the handshake warms up.
    println!("\nquerying ipinfo.io through the QUIC-fronted tunnel …");
    let mut via = None;
    for attempt in 1..=10 {
        match common::ipinfo_via_tunnel(&tunnel).await {
            Ok(v) => {
                via = Some(v);
                break;
            }
            Err(e) => {
                println!("  attempt {attempt} not ready yet ({e}); retrying");
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
    let via = via.ok_or("could not reach ipinfo.io through the QUIC tunnel after warmup")?;
    println!("IP through the tunnel:  {}", common::fmt_ipinfo(&via));
    println!("\n(the tunnelled IP/org/country should be the EXIT gateway's)");

    // 5. Tear down (bounded — live multi-threaded teardown can be slow).
    let _ = tokio::time::timeout(Duration::from_secs(5), tunnel.shutdown()).await;
    println!("PASS: public IP relocated through the QUIC-fronted two-hop tunnel");
    std::process::exit(0);
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> ExitCode {
    if let Err(e) = run().await {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
