// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Inspecting stream liveness with `last_peer_activity`.
//!
//! `last_peer_activity` reports when the peer was last heard from. It reads
//! local state only and sends nothing, so it is the passive companion to
//! automatic keepalive (which fails a truly dead peer with
//! `PeerUnresponsive`): a consumer can watch idle time and apply its own
//! policy on top.
//!
//! Two clients hold a short conversation while Alice polls the getter after
//! each exchange (idle time stays near zero). Then Bob disconnects and Alice
//! keeps polling (idle time climbs).
//!
//! ## What this demonstrates
//!
//! - `stream.wait_established(timeout)` confirms the peer accepted the stream
//! - each inbound frame moves `last_peer_activity()` forward, so a busy
//!   stream always reports a tiny idle time
//! - once the peer goes silent, `.elapsed()` on that instant grows, which is
//!   the signal a consumer watches
//!
//! Keepalive is armed on this stream, so Alice would ping Bob every 60 s.
//! Bob disconnects here, so no pong returns to reset the idle clock and the
//! reported time grows cleanly.
//!
//! ```sh
//! cargo run --example stream_liveness
//! ```

use nym_sdk::mixnet;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const TIMEOUT: Duration = Duration::from_secs(60);
const EXCHANGES: usize = 3;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Alice holds the stream we inspect; Bob is the peer that talks, then leaves.
    let mut alice = mixnet::MixnetClient::connect_new().await.unwrap();
    println!("Alice address: {}", alice.nym_address());

    let mut bob = mixnet::MixnetClient::connect_new().await.unwrap();
    let bob_addr = *bob.nym_address();
    println!("Bob address: {bob_addr}");

    let mut listener = bob.listener().unwrap();

    let mut stream = alice.open_stream(bob_addr, None).await.unwrap();
    println!("\nAlice opened stream: {}", stream.id());

    let mut inbound = tokio::time::timeout(TIMEOUT, listener.accept())
        .await
        .expect("timed out waiting for Bob to accept")
        .expect("listener shut down");
    println!("Bob accepted: {}", inbound.id());

    stream
        .wait_established(TIMEOUT)
        .await
        .expect("stream not established");
    println!("Stream established\n");

    // Phase 1: a real conversation. After each round trip the peer was just
    // heard from, so the idle time the getter reports stays near zero.
    let mut buf = vec![0u8; 1024];
    for i in 1..=EXCHANGES {
        stream.write_all(b"ping").await.unwrap();
        stream.flush().await.unwrap();

        let n = tokio::time::timeout(TIMEOUT, inbound.read(&mut buf))
            .await
            .expect("Bob timed out reading")
            .expect("Bob read failed");
        assert_eq!(&buf[..n], b"ping");
        inbound.write_all(b"pong").await.unwrap();
        inbound.flush().await.unwrap();

        let n = tokio::time::timeout(TIMEOUT, stream.read(&mut buf))
            .await
            .expect("Alice timed out reading reply")
            .expect("Alice read failed");
        assert_eq!(&buf[..n], b"pong");

        report_idle(&format!("exchange {i}/{EXCHANGES}"), &stream).await;
    }

    // Phase 2: Bob leaves. Nothing more arrives at Alice, so the last-heard
    // instant stops moving and the idle time climbs on every poll.
    println!("\nBob disconnects; watching Alice's view go stale...");
    drop(inbound);
    bob.disconnect().await;

    for _ in 0..5 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        report_idle("bob gone", &stream).await;
    }

    drop(stream);
    alice.disconnect().await;
}

/// Print how long since the peer was last heard from. `None` means the
/// stream is no longer registered, so there is nothing left to report.
async fn report_idle(label: &str, stream: &mixnet::MixnetStream) {
    match stream.last_peer_activity().await {
        Some(last) => println!("[{label}] peer last heard from {:?} ago", last.elapsed()),
        None => println!("[{label}] stream no longer registered"),
    }
}
