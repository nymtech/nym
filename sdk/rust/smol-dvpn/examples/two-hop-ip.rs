// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! `two-hop-ip` — show that a two-hop dVPN tunnel relocates your public IP.
//!
//! Queries `ipinfo.io` directly to show your real IP/location, brings up a
//! two-hop tunnel (random entry + exit gateways), prints both gateways' details,
//! then queries `ipinfo.io` again *through the tunnel* — the reported IP/org/
//! country should now be the exit gateway's.
//!
//! Usage:
//!   MNEMONIC="<funded mnemonic>" \
//!   cargo run -p nym-smol-dvpn --example two-hop-ip [-- --quic]
//!
//! `--quic` requires the entry gateway to be QUIC-bridge-capable (fails if none
//! is available for the chosen country/identity) and fronts the entry leg with
//! the QUIC bridge.

use std::process::ExitCode;
use std::time::Duration;

use nym_sdk_session::GatewaySpec;

#[path = "common/mod.rs"]
mod common;

async fn run() -> Result<(), common::BoxError> {
    common::init_crypto();
    let use_quic = std::env::args().any(|a| a == "--quic");

    // 1. Real IP (direct).
    let real = common::ipinfo_direct().await?;
    println!("real IP (no tunnel):    {}", common::fmt_ipinfo(&real));

    // 2. Provision + register a two-hop tunnel with random gateways.
    println!(
        "\nprovisioning a two-hop tunnel{} …",
        if use_quic { " (QUIC entry)" } else { "" }
    );
    let session = common::new_session("two-hop-ip-data").await;
    session.ensure_ticketbooks(true).await?;
    let reg = if use_quic {
        session
            .register_two_hop_quic(&GatewaySpec::Random, &GatewaySpec::Random)
            .await?
    } else {
        session
            .register_two_hop(&GatewaySpec::Random, &GatewaySpec::Random)
            .await?
    };

    common::print_gateway("entry", &reg.entry.gateway);
    common::print_gateway(
        "exit",
        &reg.exit.as_ref().expect("two-hop has an exit hop").gateway,
    );

    // 3. Bring up the tunnel (QUIC entry when requested).
    let tunnel = common::build_two_hop_tunnel(&reg, use_quic).await?;

    // 4. IP through the tunnel — retry while the WireGuard handshake warms up.
    println!("\nquerying ipinfo.io through the tunnel …");
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
    let via = via.ok_or("could not reach ipinfo.io through the tunnel after warmup")?;
    println!("IP through the tunnel:  {}", common::fmt_ipinfo(&via));
    println!("\n(the tunnelled IP/org/country should be the EXIT gateway's)");

    // 5. Tear down (bounded — live multi-threaded teardown can be slow).
    let _ = tokio::time::timeout(Duration::from_secs(5), tunnel.shutdown()).await;
    println!("PASS: public IP relocated through the two-hop tunnel");
    // Guarantee prompt termination regardless of background reactor teardown.
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
