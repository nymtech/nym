// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Sends a 1 MB random file over a MixnetStream and verifies the
//! receiver got an identical copy. Cancel with Ctrl+C.
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

    // Generate random payload
    let mut payload = vec![0u8; SIZE];
    rand::rngs::OsRng.fill_bytes(&mut payload);
    println!("Generated {} bytes of random data", payload.len());

    // Connect two clients
    println!("Connecting sender...");
    let mut sender = mixnet::MixnetClient::connect_new().await.unwrap();
    println!("{}", sender.nym_address());

    println!("Connecting receiver...");
    let mut receiver = mixnet::MixnetClient::connect_new().await.unwrap();
    let recv_addr = *receiver.nym_address();
    println!("{recv_addr}");

    // Open stream
    let mut listener = receiver.listener().unwrap();
    let mut tx = sender.open_stream(recv_addr, None).await.unwrap();
    let mut rx = tokio::time::timeout(TIMEOUT, listener.accept())
        .await
        .expect("accept timed out")
        .expect("listener closed");
    println!("Stream established\n");

    // Send
    let data = payload.clone();
    let send_task = tokio::spawn(async move {
        tx.write_all(&data).await.unwrap();
        tx.flush().await.unwrap();
        println!("Sent {} bytes", data.len());
    });

    // Receive — read exactly SIZE bytes (don't rely on close/EOF - if we need this in future
    // iterations we can introduce something like what the TcpProxy module has)
    let recv_task = tokio::spawn(async move {
        let mut buf = vec![0u8; SIZE];
        tokio::time::timeout(TIMEOUT, rx.read_exact(&mut buf))
            .await
            .expect("receive timed out")
            .unwrap();
        println!("Received {} bytes", buf.len());
        buf
    });

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
