// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Demonstrates concurrent streams over the Mixnet.
//!
//! One sender opens streams to two receivers.
//! Both receivers accept, read, and reply concurrently.
//!
//! Run with: cargo run --example async_read_write

use nym_sdk::mixnet;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_tracing_logger();

    let mut sender = mixnet::MixnetClient::connect_new().await.unwrap();
    println!("Sender address: {}", sender.nym_address());

    let mut receiver_a = mixnet::MixnetClient::connect_new().await.unwrap();
    let addr_a = *receiver_a.nym_address();
    println!("Receiver A address: {addr_a}");

    let mut receiver_b = mixnet::MixnetClient::connect_new().await.unwrap();
    let addr_b = *receiver_b.nym_address();
    println!("Receiver B address: {addr_b}");

    let mut listener_a = receiver_a.listener().unwrap();
    let mut listener_b = receiver_b.listener().unwrap();

    println!("\nOpening streams to both receivers...");
    let mut stream_to_a = sender.open_stream(addr_a, None).await.unwrap();
    println!("Stream to A opened: {}", stream_to_a.id());

    let mut stream_to_b = sender.open_stream(addr_b, None).await.unwrap();
    println!("Stream to B opened: {}", stream_to_b.id());

    println!("\nWaiting for both receivers to accept...");
    let (inbound_a, inbound_b) = tokio::try_join!(
        async {
            tokio::time::timeout(TIMEOUT, listener_a.accept())
                .await
                .expect("timed out waiting for A to accept")
                .ok_or("listener A shut down")
        },
        async {
            tokio::time::timeout(TIMEOUT, listener_b.accept())
                .await
                .expect("timed out waiting for B to accept")
                .ok_or("listener B shut down")
        },
    )
    .unwrap();
    println!("A accepted stream: {}", inbound_a.id());
    println!("B accepted stream: {}", inbound_b.id());

    let msg_a = b"hello receiver A";
    let msg_b = b"hello receiver B";

    println!("\nSender writing to both streams...");
    stream_to_a.write_all(msg_a).await.unwrap();
    stream_to_a.flush().await.unwrap();

    stream_to_b.write_all(msg_b).await.unwrap();
    stream_to_b.flush().await.unwrap();

    println!("\nBoth receivers reading and replying concurrently...");
    let reply_a = b"reply from A";
    let reply_b = b"reply from B";

    let (res_a, res_b) = tokio::join!(
        // Receiver A: read then reply
        async {
            let mut inbound = inbound_a;
            let mut buf = vec![0u8; 1024];
            let n = tokio::time::timeout(TIMEOUT, inbound.read(&mut buf))
                .await
                .expect("A: timed out reading")
                .expect("A: read failed");
            println!("Receiver A got: {:?}", String::from_utf8_lossy(&buf[..n]));
            assert_eq!(&buf[..n], msg_a);

            inbound.write_all(reply_a).await.unwrap();
            inbound.flush().await.unwrap();
            println!("Receiver A replied");
            inbound
        },
        // Receiver B: read then reply
        async {
            let mut inbound = inbound_b;
            let mut buf = vec![0u8; 1024];
            let n = tokio::time::timeout(TIMEOUT, inbound.read(&mut buf))
                .await
                .expect("B: timed out reading")
                .expect("B: read failed");
            println!("Receiver B got: {:?}", String::from_utf8_lossy(&buf[..n]));
            assert_eq!(&buf[..n], msg_b);

            inbound.write_all(reply_b).await.unwrap();
            inbound.flush().await.unwrap();
            println!("Receiver B replied");
            inbound
        },
    );
    let inbound_a = res_a;
    let inbound_b = res_b;

    println!("\nSender reading replies...");
    tokio::join!(
        async {
            let mut buf = vec![0u8; 1024];
            let n = tokio::time::timeout(TIMEOUT, stream_to_a.read(&mut buf))
                .await
                .expect("timed out reading reply from A")
                .expect("read failed");
            println!(
                "Sender got from A: {:?}",
                String::from_utf8_lossy(&buf[..n])
            );
            assert_eq!(&buf[..n], reply_a);
        },
        async {
            let mut buf = vec![0u8; 1024];
            let n = tokio::time::timeout(TIMEOUT, stream_to_b.read(&mut buf))
                .await
                .expect("timed out reading reply from B")
                .expect("read failed");
            println!(
                "Sender got from B: {:?}",
                String::from_utf8_lossy(&buf[..n])
            );
            assert_eq!(&buf[..n], reply_b);
        },
    );

    println!("\nConcurrent round-trips successful!");

    // Streams clean up on drop (unregister from router).
    // No close message is sent over the wire — see stream.rs.
    drop(stream_to_a);
    drop(stream_to_b);
    drop(inbound_a);
    drop(inbound_b);
    sender.disconnect().await;
    receiver_a.disconnect().await;
    receiver_b.disconnect().await;
}
