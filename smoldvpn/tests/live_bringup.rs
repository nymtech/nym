// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Live integration test (OpenSpec task 4.11): bring up single-hop and two-hop
//! tunnels against real Nym gateways and pass traffic through them.
//!
//! `#[ignore]` by default because it needs a funded mnemonic + network access to
//! a live Nym network (sandbox). Run it with the sandbox env + secrets sourced:
//!
//! ```sh
//! set -a; source envs/sandbox.env; source .claude/.secrets/sandbox.env; set +a
//! MNEMONIC="$NYX_ACCOUNT_MNEMONIC" \
//!   cargo test -p nym-smoldvpn --test live_bringup -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Run with `--test-threads=1`: both tests deposit from the same chain account, so running
//! them concurrently races the account sequence number (one deposit fails with an
//! "account sequence mismatch"). Serially, both pass.
//!
//! It also demonstrates the intended `nym-sdk-session` → `smoldvpn` glue:
//! mapping a `Registration`'s per-hop `HopConfig` into the datapath `PeerConfig`.

use std::time::Duration;

use std::net::IpAddr;

use nym_network_defaults::NymNetworkDetails;
use nym_sdk_session::{GatewaySpec, HopConfig, Registration, Session, SessionConfig};
use nym_smoldvpn::{MtuConfig, PeerConfig, Tunnel, TunnelBuilder};
use tokio_util::sync::CancellationToken;

/// Map a session hop into the datapath's transport-agnostic peer config.
fn peer_from_hop(hop: HopConfig) -> PeerConfig {
    PeerConfig {
        gateway_public_key: hop.wg_config.public_key,
        client_private_key: hop.client_private_key,
        preshared_key: hop.wg_config.psk.as_ref().map(|p| *p.as_bytes()),
        endpoint: hop.wg_config.endpoint,
        assigned_ipv4: hop.wg_config.private_ipv4,
        assigned_ipv6: Some(hop.wg_config.private_ipv6),
    }
}

fn mnemonic() -> Option<bip39::Mnemonic> {
    let mnemonic = std::env::var("MNEMONIC")
        .or_else(|_| std::env::var("NYX_ACCOUNT_MNEMONIC"))
        .inspect_err(|_| eprintln!("set MNEMONIC or NYX_ACCOUNT_MNEMONIC to run this test"))
        .ok()?
        .parse()
        .expect("valid bip39 mnemonic");
    Some(mnemonic)
}

async fn new_session(data_dir: &str) -> Option<Session> {
    let cancel = CancellationToken::new();
    let session = Session::new(
        SessionConfig {
            mnemonic: mnemonic()?,
            network: NymNetworkDetails::new_from_env(),
            credential_store_path: Some(format!("{data_dir}/creds.db").into()),
            data_path: data_dir.into(),
            dvpn_directory_url: None,
            automatic_topups: None,
            bandwidth_provider: None,
            reuse_registrations: true,
        },
        cancel,
    )
    .await
    .expect("session init");
    Some(session)
}

/// Resolve a hostname through the tunnel, retrying while the WireGuard handshake
/// (or a freshly rebuilt interface) warms up.
async fn resolve_with_warmup(tunnel: &Tunnel) -> Vec<IpAddr> {
    for attempt in 1..=10 {
        match tunnel.resolve("nymtech.net").await {
            Ok(addrs) if !addrs.is_empty() => return addrs,
            Ok(_) | Err(_) => {
                println!("resolve attempt {attempt} not ready yet; retrying");
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
    Vec::new()
}

/// Bring up `registration` as a tunnel and prove traffic flows by resolving a
/// hostname through it (exercises the UDP socket + smol-core DNS path), then
/// exercise a runtime MTU change (task 4.7) and prove traffic still flows.
async fn probe_traffic(builder: TunnelBuilder) {
    let tunnel: Tunnel = builder.connect().await.expect("tunnel connect");

    let resolved = resolve_with_warmup(&tunnel).await;
    assert!(
        !resolved.is_empty(),
        "no addresses resolved through the tunnel after warmup"
    );
    println!("resolved nymtech.net through the tunnel: {resolved:?}");

    // Runtime MTU change (task 4.7): rebuild the interface at the MOBILE MTU
    // while keeping the WireGuard session, then confirm traffic still flows.
    tunnel
        .set_mtu(MtuConfig::MOBILE)
        .expect("set_mtu at runtime");
    println!("changed MTU at runtime to {:?}", tunnel.mtu());
    let after = resolve_with_warmup(&tunnel).await;
    assert!(
        !after.is_empty(),
        "no addresses resolved through the tunnel after runtime MTU change"
    );
    println!("resolved again after MTU change: {after:?}");

    // Traffic is proven; tear down (bounded — teardown of a live multi-threaded
    // runtime can be slow, so we don't fail the proven test on shutdown latency).
    // The timeout bounds teardown latency without `process::exit`, which would kill any sibling
    // test sharing this binary (run these with `--test-threads=1` — see the module docs).
    let _ = tokio::time::timeout(Duration::from_secs(5), tunnel.shutdown()).await;
    println!("PASS: traffic flowed through the tunnel (incl. after MTU change)");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires a funded mnemonic + live Nym network (sandbox)"]
async fn single_hop_bringup_passes_traffic() {
    let Some(session) = new_session("live-single").await else {
        eprintln!("could not run the test without valid session");
        return;
    };
    session
        .ensure_ticketbooks(false)
        .await
        .expect("issue ticketbooks");
    let reg: Registration = session
        .register_single_hop(&GatewaySpec::Random)
        .await
        .expect("single-hop registration");

    let peer = peer_from_hop(reg.entry);
    probe_traffic(TunnelBuilder::single_hop(peer)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires a funded mnemonic + live Nym network (sandbox)"]
async fn two_hop_bringup_passes_traffic() {
    let Some(session) = new_session("live-two").await else {
        eprintln!("could not run the test without valid session");
        return;
    };
    session
        .ensure_ticketbooks(true)
        .await
        .expect("issue ticketbooks");
    let reg: Registration = session
        .register_two_hop(&GatewaySpec::Random, &GatewaySpec::Random)
        .await
        .expect("two-hop registration");

    let entry = peer_from_hop(reg.entry);
    let exit = peer_from_hop(reg.exit.expect("two-hop must have an exit hop"));
    println!(
        "entry: endpoint={} assigned_ipv4={}",
        entry.endpoint, entry.assigned_ipv4
    );
    println!(
        "exit:  endpoint={} assigned_ipv4={}",
        exit.endpoint, exit.assigned_ipv4
    );
    probe_traffic(TunnelBuilder::two_hop(entry, exit)).await;
}
