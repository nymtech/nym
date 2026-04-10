// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Demonstrates stream idle timeout cleanup.
//!
//! Opens a stream to self, uses it, then stops. After the idle timeout
//! elapses the router removes the stream and reads return EOF.
//!
//! Run with: cargo run --example stream_idle_timeout

use nym_sdk::mixnet;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Short idle timeout so we don't wait the default
const IDLE_TIMEOUT: Duration = Duration::from_secs(2);
const WAIT_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Build a client with a short stream idle timeout (default is 30 min).
    let mut client = mixnet::MixnetClientBuilder::new_ephemeral()
        .with_stream_idle_timeout(IDLE_TIMEOUT)
        .build()
        .unwrap()
        .connect_to_mixnet()
        .await
        .unwrap();

    let our_address = *client.nym_address();
    println!("Client address: {our_address}");

    // Open a stream to ourselves (useful for testing).
    let mut listener = client.listener().unwrap();
    let mut outbound = client.open_stream(our_address, None).await.unwrap();
    println!("Opened outbound stream: {}", outbound.id());

    let mut inbound = tokio::time::timeout(WAIT_TIMEOUT, listener.accept())
        .await
        .expect("timed out waiting for accept")
        .expect("listener shut down");
    println!("Accepted inbound stream: {}", inbound.id());

    // Use the stream — send and receive.
    let msg = b"hello from idle timeout example";
    outbound.write_all(msg).await.unwrap();
    outbound.flush().await.unwrap();

    let mut buf = vec![0u8; 1024];
    let n = tokio::time::timeout(WAIT_TIMEOUT, inbound.read(&mut buf))
        .await
        .expect("timed out reading")
        .expect("read failed");
    println!("Received: {:?}", String::from_utf8_lossy(&buf[..n]));
    assert_eq!(&buf[..n], msg);

    // Stop using the stream. The router's periodic cleanup task
    // will remove it after the idle timeout elapses.
    println!(
        "\nStream is idle. Waiting {}s for cleanup...",
        IDLE_TIMEOUT.as_secs()
    );
    tokio::time::sleep(IDLE_TIMEOUT + Duration::from_secs(2)).await;

    // Read returns 0 bytes (EOF) — the router cleaned up the stream.
    let n = inbound.read(&mut buf).await.expect("read failed");
    if n == 0 {
        println!("Inbound stream returned EOF — cleaned up by idle timeout.");
    } else {
        println!("Unexpected: got {n} bytes after idle timeout");
    }

    drop(outbound);
    drop(inbound);
    client.disconnect().await;
}
