// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `Tunnel::await_established` (change `dvpn-registration-reuse`, task 4.3):
//! deterministic loopback tests — a minimal boringtun responder plays the
//! gateway side of the WireGuard handshake (and swallows all data traffic),
//! while a bound-but-silent UDP socket plays a dead gateway peer (the
//! signature of a stale cached registration).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::net::SocketAddr;
use std::time::Duration;

use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};
use nym_crypto::asymmetric::x25519;
use nym_smoldvpn::{NotEstablished, PeerConfig, TunnelBuilder};

/// Run an async test body on a manually-built runtime and force-stop it after.
/// The smol-core stack parks workers in tokio's blocking pool that outlive the
/// tunnel; a plain `#[tokio::test]` would hang forever in `Runtime::drop`
/// waiting for them (the examples sidestep the same issue with
/// `std::process::exit(0)`). `shutdown_timeout` abandons the stragglers.
fn run_test<F: std::future::Future>(f: F) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(f);
    rt.shutdown_timeout(Duration::from_secs(2));
}

fn keypair(seed: u8) -> (StaticSecret, PublicKey) {
    let secret = StaticSecret::from([seed; 32]);
    let public = PublicKey::from(&secret);
    (secret, public)
}

/// Build a `PeerConfig` from boringtun-side test keys (the responder speaks
/// boringtun; `PeerConfig` speaks nym-native x25519 — convert at the boundary).
fn peer(
    gateway_public_key: PublicKey,
    client_secret: StaticSecret,
    endpoint: SocketAddr,
) -> PeerConfig {
    PeerConfig {
        gateway_public_key: x25519::PublicKey::from(gateway_public_key.to_bytes()),
        client_private_key: x25519::PrivateKey::from_secret(client_secret.to_bytes()),
        preshared_key: None,
        endpoint,
        assigned_ipv4: std::net::Ipv4Addr::new(10, 1, 2, 3),
        assigned_ipv6: None,
    }
}

/// A gateway-side WireGuard peer: completes handshakes, swallows data. Runs
/// until its socket errors or the test ends (tasks are dropped with the
/// runtime).
async fn wg_responder(
    socket: tokio::net::UdpSocket,
    server_secret: StaticSecret,
    client_public: PublicKey,
) {
    let mut tunn = Tunn::new(server_secret, client_public, None, None, 0, None);
    let mut buf = [0u8; 2048];
    let mut scratch = [0u8; 4096];
    loop {
        let Ok((n, src)) = socket.recv_from(&mut buf).await else {
            break;
        };
        // Copy responses out of `scratch` before the next decapsulate reuses it.
        let mut responses: Vec<Vec<u8>> = Vec::new();
        let queued = match tunn.decapsulate(None, &buf[..n], &mut scratch) {
            TunnResult::WriteToNetwork(p) => {
                responses.push(p.to_vec());
                true
            }
            // Data packets (e.g. the client's tunnelled exit handshake) are
            // deliberately swallowed: this responder establishes its own hop
            // and forwards nothing.
            _ => false,
        };
        if queued {
            while let TunnResult::WriteToNetwork(p) = tunn.decapsulate(None, &[], &mut scratch) {
                responses.push(p.to_vec());
            }
        }
        for r in responses {
            let _ = socket.send_to(&r, src).await;
        }
    }
}

/// Spawn a responder gateway; returns its address and public key.
async fn spawn_responder(client_public: PublicKey, server_seed: u8) -> (SocketAddr, PublicKey) {
    let (server_secret, server_public) = keypair(server_seed);
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = socket.local_addr().unwrap();
    tokio::spawn(wg_responder(socket, server_secret, client_public));
    (addr, server_public)
}

/// A bound socket that never answers — a dead/forgotten gateway peer.
async fn spawn_silent() -> (SocketAddr, tokio::net::UdpSocket, PublicKey) {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = socket.local_addr().unwrap();
    let (_, public) = keypair(99);
    (addr, socket, public)
}

/// Healthy bring-up resolves promptly (spec: "Healthy bring-up resolves promptly").
#[test]
fn single_hop_establishes_against_responder() {
    run_test(async {
        let (client_secret, client_public) = keypair(1);
        let (addr, server_public) = spawn_responder(client_public, 2).await;

        let tunnel = TunnelBuilder::single_hop(peer(server_public, client_secret, addr))
            .connect()
            .await
            .expect("tunnel connect");
        tunnel
            .await_established(Duration::from_secs(5))
            .await
            .expect("must establish against a live responder");
        let _ = tokio::time::timeout(Duration::from_secs(5), tunnel.shutdown()).await;
    });
}

/// A dead peer is detected within the bound with per-hop status
/// (spec: "Dead cached registration is detected within the bound").
#[test]
fn single_hop_times_out_against_silent_peer() {
    run_test(async {
        let (client_secret, _client_public) = keypair(1);
        let (addr, _keepalive_socket, server_public) = spawn_silent().await;

        let tunnel = TunnelBuilder::single_hop(peer(server_public, client_secret, addr))
            .connect()
            .await
            .expect("tunnel connect");
        let err = tunnel
            .await_established(Duration::from_millis(400))
            .await
            .expect_err("must time out against a silent peer");
        assert_eq!(
            err,
            NotEstablished {
                entry: false,
                exit: None
            }
        );
        let _ = tokio::time::timeout(Duration::from_secs(5), tunnel.shutdown()).await;
    });
}

/// Two-hop against silent peers reports both hops down.
#[test]
fn two_hop_times_out_with_both_hops_down() {
    run_test(async {
        let (entry_secret, _) = keypair(1);
        let (exit_secret, _) = keypair(2);
        let (entry_addr, _s1, entry_public) = spawn_silent().await;
        let (exit_addr, _s2, exit_public) = spawn_silent().await;

        let tunnel = TunnelBuilder::two_hop(
            peer(entry_public, entry_secret, entry_addr),
            peer(exit_public, exit_secret, exit_addr),
        )
        .connect()
        .await
        .expect("tunnel connect");
        let err = tunnel
            .await_established(Duration::from_millis(400))
            .await
            .expect_err("must time out");
        assert_eq!(
            err,
            NotEstablished {
                entry: false,
                exit: Some(false)
            }
        );
        let _ = tokio::time::timeout(Duration::from_secs(5), tunnel.shutdown()).await;
    });
}

/// Exit-only failure is attributed to the exit hop (spec: "Exit-only failure
/// is attributed to the exit hop"): the entry responder completes ITS
/// handshake but swallows the tunnelled exit handshake, so the exit never
/// establishes — exactly what a caller needs to invalidate only the exit's
/// cached registration.
#[test]
fn two_hop_exit_failure_is_attributed_to_exit() {
    run_test(async {
        let (entry_secret, entry_client_public) = keypair(1);
        let (exit_secret, _) = keypair(2);
        let (entry_addr, entry_public) = spawn_responder(entry_client_public, 3).await;
        // The exit endpoint only matters as an inner-frame destination; its
        // handshake is swallowed by the entry responder.
        let (exit_addr, _s, exit_public) = spawn_silent().await;

        let tunnel = TunnelBuilder::two_hop(
            peer(entry_public, entry_secret, entry_addr),
            peer(exit_public, exit_secret, exit_addr),
        )
        .connect()
        .await
        .expect("tunnel connect");
        let err = tunnel
            .await_established(Duration::from_secs(1))
            .await
            .expect_err("exit must not establish");
        assert_eq!(
            err,
            NotEstablished {
                entry: true,
                exit: Some(false)
            }
        );
        let _ = tokio::time::timeout(Duration::from_secs(5), tunnel.shutdown()).await;
    });
}
