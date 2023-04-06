//! The mixnet component of the Rust SDK for the Nym platform
//!
//!
//! # Basic example
//!
//! ```no_run
//! use nym_sdk::mixnet;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Passing no config makes the client fire up an ephemeral session and figure stuff out on
//!     // its own
//!     let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
//!
//!     // Be able to get our client address
//!     let our_address = client.nym_address();
//!     println!("Our client nym address is: {our_address}");
//!
//!     // Send a message throught the mixnet to ourselves
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
mod config;
mod connection_state;
mod keys;
mod native_client;
mod paths;
mod socks5_client;

pub use client::{DisconnectedMixnetClient, IncludedSurbs, MixnetClientBuilder};
pub use config::Config;
pub use keys::{Keys, KeysArc};
pub use native_client::MixnetClient;
pub use native_client::MixnetClientSender;
pub use nym_client_core::{
    client::{
        inbound_messages::InputMessage,
        replies::reply_storage::{fs_backend::Backend as ReplyStorage, Empty as EmptyReplyStorage},
    },
    config::GatewayEndpointConfig,
};
pub use nym_socks5_client_core::config::Socks5;
pub use nym_sphinx::{
    addressing::clients::{ClientIdentity, Recipient},
    receiver::ReconstructedMessage,
};
pub use nym_topology::{provider_trait::TopologyProvider, NymTopology};
pub use paths::{GatewayKeyMode, KeyMode, StoragePaths};
pub use socks5_client::Socks5MixnetClient;
