// Copyright 2024 Nym Technologies SA
// SPDX-License-Identifier: Apache-2.0

//! libp2p transport over Nym mixnet for WASM/browser environments.
//!
//! This crate provides a libp2p Transport implementation that uses the Nym mixnet
//! for privacy-preserving peer-to-peer communication in browser environments.
//!
//! # Features
//!
//! - Full libp2p Transport trait implementation
//! - Stream multiplexing via StreamMuxer
//! - Message ordering over the unordered mixnet
//! - WASM/browser compatible (uses futures channels, gloo_timers)
//!
//! # Example
//!
//! ```ignore
//! use nym_libp2p_wasm::{create_transport_client_async, NymTransport};
//! use libp2p_identity::Keypair;
//!
//! async fn example() {
//!     // Create the transport client (connects to Nym network)
//!     let result = create_transport_client_async(None).await.unwrap();
//!
//!     // Create the transport
//!     let keypair = Keypair::generate_ed25519();
//!     let transport = NymTransport::new(
//!         result.self_address,
//!         result.stream,
//!         keypair,
//!     ).await.unwrap();
//!     // Use transport with libp2p Swarm...
//! }
//! ```

#[cfg(target_arch = "wasm32")]
pub mod client;
#[cfg(target_arch = "wasm32")]
pub(crate) mod connection;
#[cfg(target_arch = "wasm32")]
pub mod error;
#[cfg(target_arch = "wasm32")]
pub(crate) mod message;
#[cfg(target_arch = "wasm32")]
pub(crate) mod mixnet;
#[cfg(target_arch = "wasm32")]
pub(crate) mod queue;
#[cfg(target_arch = "wasm32")]
pub mod substream;
#[cfg(target_arch = "wasm32")]
pub mod transport;

// Re-exports for convenience
#[cfg(target_arch = "wasm32")]
pub use client::{create_transport_client_async, TransportClient, TransportClientOpts};
#[cfg(target_arch = "wasm32")]
pub use connection::Connection;
#[cfg(target_arch = "wasm32")]
pub use error::Error;
#[cfg(target_arch = "wasm32")]
pub use nym_sphinx_addressing::clients::Recipient;
#[cfg(target_arch = "wasm32")]
pub use substream::Substream;
pub use transport::{nym_address_to_multiaddress, NymTransport, Upgrade};

/// The default timeout in seconds for the transport upgrade/handshake.
#[cfg(target_arch = "wasm32")]
const DEFAULT_HANDSHAKE_TIMEOUT_SECS: u64 = 30;
