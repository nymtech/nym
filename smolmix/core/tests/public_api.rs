// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! Compile-time lock on `smolmix`'s public API.
//!
//! `smolmix` is published (1.21.3) and its public surface must be preserved
//! across the refactor onto `smol-core` (OpenSpec task 2.5). If any of these
//! items is renamed, removed, or changes shape, this test fails to compile —
//! catching an accidental API break without needing a live mixnet. Behavioural
//! regression is covered by the crate's examples run against a live mixnet.

use nym_smolmix::{IpPair, Recipient, SmolmixError, TcpStream, Tunnel, TunnelBuilder, UdpSocket};

// Every referenced path must exist with its current name and arity.
#[allow(unused, clippy::no_effect)]
fn _api_lock() {
    // Tunnel constructors + lifecycle.
    let _ = Tunnel::builder;
    let _ = Tunnel::new;
    let _ = Tunnel::new_with_ipr;
    let _ = Tunnel::from_stream;
    let _ = Tunnel::tcp_connect;
    let _ = Tunnel::udp_socket;
    let _ = Tunnel::udp_socket_on;
    let _ = Tunnel::allocated_ips;
    let _ = Tunnel::shutdown;

    // Builder.
    let _ = TunnelBuilder::ipr_address;
    let _ = TunnelBuilder::build;
}

// The public types must remain nameable and `Tunnel` cloneable + thread-safe.
#[allow(unused)]
fn _types(_ip: IpPair, _r: Recipient, _e: SmolmixError, _t: TcpStream, _u: UdpSocket) {}

#[test]
fn public_api_is_stable() {
    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_clone<T: Clone>() {}
    assert_send_sync::<Tunnel>();
    assert_clone::<Tunnel>();
}
