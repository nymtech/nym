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

// WASM-bindgen exports for JavaScript
#[cfg(target_arch = "wasm32")]
mod js_api {
    use js_sys::Promise;
    use libp2p_identity::Keypair;
    use nym_wasm_utils::console_log;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::future_to_promise;

    use crate::client::{create_transport_client_async, TransportClientOpts};
    use crate::transport::NymTransport;

    /// Initialize and test the libp2p-nym transport.
    /// Returns a Promise that resolves with the Nym address on success.
    ///
    /// @param force_tls - If true, forces TLS connections to gateways (required for browsers)
    /// @param client_id - Optional client ID for storage namespace (defaults to "libp2p-transport")
    #[wasm_bindgen(js_name = "testTransport")]
    pub fn test_transport(
        nym_api_url: Option<String>,
        force_tls: Option<bool>,
        client_id: Option<String>,
    ) -> Promise {
        future_to_promise(async move {
            let opts = TransportClientOpts {
                nym_api_url,
                force_tls: force_tls.unwrap_or(true), // Default to true for browser safety
                client_id,                            // Uses "libp2p-transport" if None
            };
            console_log!("testTransport: force_tls={}", opts.force_tls);

            let result = create_transport_client_async(opts)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            let address = result.self_address.to_string();
            console_log!("Client connected! Address: {}", address);

            // Create a libp2p keypair
            let keypair = Keypair::generate_ed25519();
            console_log!("Generated libp2p keypair");

            // Create the transport
            let _transport = NymTransport::new(result.self_address, result.stream, keypair)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            console_log!("Transport created successfully!");
            console_log!("Listening on: /nym/{}", address);

            Ok(JsValue::from_str(&address))
        })
    }
}
