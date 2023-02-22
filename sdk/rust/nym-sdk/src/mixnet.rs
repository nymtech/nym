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
mod paths;

pub use client::{
    DisconnectedMixnetClient, IncludedSurbs, MixnetClient, MixnetClientBuilder, MixnetClientSender,
};
pub use client_core::{
    client::{
        inbound_messages::InputMessage,
        replies::reply_storage::{fs_backend::Backend as ReplyStorage, Empty as EmptyReplyStorage},
    },
    config::GatewayEndpointConfig,
};
pub use nym_config::Config;
pub use keys::{Keys, KeysArc};
pub use nym_sphinx::{
    addressing::clients::{ClientIdentity, Recipient},
    receiver::ReconstructedMessage,
};
pub use paths::{GatewayKeyMode, KeyMode, StoragePaths};
