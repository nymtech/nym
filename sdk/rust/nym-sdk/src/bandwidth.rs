// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
//! The coconut bandwidth component of the Rust SDK for the Nym platform
//!
//!
//! # Basic example
//!
//! ```no_run
//! use nym_sdk::mixnet;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mixnet_client = mixnet::MixnetClientBuilder::new()
//!         .enable_credentials_mode()
//!         .build::<mixnet::EmptyReplyStorage>()
//!         .await
//!         .unwrap();
//!
//!     let bandwidth_client = mixnet_client.create_bandwidth_client(String::from("my super secret mnemonic")).unwrap();
//!
//!     // Get a bandwidth credential worth 1000000 unym for the mixnet_client
//!     bandwidth_client.acquire(1000000).await.unwrap();
//!
//!     // Connect using paid bandwidth credential
//!     let mut client = mixnet_client.connect_to_mixnet().await.unwrap();
//!
//!     let our_address = client.nym_address();
//!
//!     // Send a message throughout the mixnet to ourselves
//!     client.send_str(*our_address, "hello there").await;
//!
//!     println!("Waiting for message");
//!     if let Some(received) = client.wait_for_messages().await {
//!         for r in received {
//!             println!("Received: {}", String::from_utf8_lossy(&r.message));
//!         }
//!     }
//!
//!     client.disconnect().await;
//! }
//! ```

mod client;

pub use client::{BandwidthAcquireClient, VoucherBlob};
