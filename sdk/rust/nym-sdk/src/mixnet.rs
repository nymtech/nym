//! The mixnet component of the Rust SDK for the Nym platform
//!
//!
//! # Basic example
//!
//! ```rust
//! use nym_sdk::mixnet;
//!
//! #[tokio::main]
//! async fn main() {
//!     logging::setup_logging();
//!
//!     // Passing no config makes the client fire up an ephemeral session and figure stuff out on
//!     // its own
//!     let mut client = mixnet::MixnetClient::connect().await.unwrap();
//!
//!     // Be able to get our client address
//!     let our_address = client.nym_address();
//!     println!("Our client nym address is: {our_address}");
//!
//!     // Send a message throught the mixnet to ourselves
//!     client
//!         .send_str(&our_address.to_string(), "hello there")
//!         .await;
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
mod config;
mod connection_state;
mod keys;
mod paths;

#[doc(inline)]
pub use client_core::config::GatewayEndpointConfig;
pub use nymsphinx::{
    addressing::clients::{ClientIdentity, Recipient},
    receiver::ReconstructedMessage,
};

#[doc(inline)]
pub use keys::{Keys, KeysArc};
pub use paths::{GatewayKeyMode, KeyMode, StoragePaths};

pub use client::{MixnetClient, MixnetClientBuilder};
pub use config::Config;
