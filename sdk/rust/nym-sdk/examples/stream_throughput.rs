// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Sends 1 MB of random data over a MixnetStream and verifies integrity.
//!
//! Uses `write_all` on the sender and `read_exact` on the receiver.
//! `read_exact` is needed because there is no close/EOF signal — streams
//! clean up via `Drop` and idle timeout, so the receiver must know the
//! expected size up front.
//!
//! Messages are reordered by sequence number in the stream layer, so
//! large payloads spanning multiple Sphinx packets are reassembled
//! correctly.
//!
//! Run with: cargo run --example stream_throughput

use nym_sdk::mixnet;
use rand::RngCore;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SIZE: usize = 1024 * 1024; // 1 MB
const TIMEOUT: Duration = Duration::from_secs(300);

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    // Step 1: Generate 1 MB of random data to send.
    let mut payload = vec![0u8; SIZE];
    rand::rngs::OsRng.fill_bytes(&mut payload);
    println!("Generated {} bytes of random data", payload.len());

    // Step 2: Connect two clients and establish a stream.
    println!("Connecting sender...");
    let mut sender = mixnet::MixnetClient::connect_new().await.unwrap();
    println!("{}", sender.nym_address());

    println!("Connecting receiver...");
    let mut receiver = mixnet::MixnetClient::connect_new().await.unwrap();
    let recv_addr = *receiver.nym_address();
    println!("{recv_addr}");

    let mut listener = receiver.listener().unwrap();
    let mut tx = sender.open_stream(recv_addr, None).await.unwrap();
    let mut rx = tokio::time::timeout(TIMEOUT, listener.accept())
        .await
        .expect("accept timed out")
        .expect("listener closed");
    println!("Stream established\n");

    // Step 3: Send the payload. write_all splits it across multiple
    // Sphinx packets automatically.
    let data = payload.clone();
    let send_task = tokio::spawn(async move {
        tx.write_all(&data).await.unwrap();
        tx.flush().await.unwrap();
        println!("Sent {} bytes", data.len());
    });

    // Step 4: Receive exactly SIZE bytes using read_exact.
    // We use read_exact (not read-until-EOF) because there is no
    // close/EOF signal — streams clean up via Drop and idle timeout.
    let recv_task = tokio::spawn(async move {
        let mut buf = vec![0u8; SIZE];
        tokio::time::timeout(TIMEOUT, rx.read_exact(&mut buf))
            .await
            .expect("receive timed out")
            .unwrap();
        println!("Received {} bytes", buf.len());
        buf
    });

    // Step 5: Verify integrity — the received bytes must match exactly.
    let (_, received) = tokio::join!(send_task, recv_task);
    let received = received.unwrap();

    if received == payload {
        println!("\nIntegrity OK");
    } else {
        eprintln!(
            "\nMISMATCH — sent {} bytes, got {}",
            payload.len(),
            received.len()
        );
        std::process::exit(1);
    }

    sender.disconnect().await;
    receiver.disconnect().await;
}
