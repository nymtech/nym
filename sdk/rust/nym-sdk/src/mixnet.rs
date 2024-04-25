//! The mixnet component of the Rust SDK for the Nym platform
//!
//!
//! # Basic example
//!
//! ```no_run
//! use nym_sdk::mixnet::{self, MixnetMessageSender};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Passing no config makes the client fire up an ephemeral session and figure stuff out on
//!     // its own
//! let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
//!
//!     // Be able to get our client address
//!     let our_address = client.nym_address();
//!     println!("Our client nym address is: {our_address}");
//!
//!     // Send a message throught the mixnet to ourselves
//!     client.send_plain_message(*our_address, "hello there").await.unwrap();
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
mod native_client;
mod paths;
mod socks5_client;
mod traits;

pub use client::{DisconnectedMixnetClient, IncludedSurbs, MixnetClientBuilder};
pub use config::Config;
pub use native_client::MixnetClient;
pub use native_client::MixnetClientSender;
pub use nym_client_core::{
    client::{
        base_client::storage::{
            gateways_storage::{
                ActiveGateway, BadGateway, GatewayRegistration, GatewaysDetailsStore,
            },
            Ephemeral, MixnetClientStorage, OnDiskPersistent,
        },
        inbound_messages::InputMessage,
        key_manager::{
            persistence::{InMemEphemeralKeys, KeyStore, OnDiskKeys},
            ClientKeys,
        },
        replies::reply_storage::{
            fs_backend::Backend as ReplyStorage, CombinedReplyStorage, Empty as EmptyReplyStorage,
            ReplyStorageBackend,
        },
        topology_control::geo_aware_provider::{CountryGroup, GeoAwareTopologyProvider},
    },
    config::GroupBy,
};
pub use nym_credential_storage::{
    ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage,
    models::StoredIssuedCredential, storage::Storage as CredentialStorage,
};
pub use nym_network_defaults::NymNetworkDetails;
pub use nym_socks5_client_core::config::Socks5;
pub use nym_sphinx::{
    addressing::{
        clients::{ClientIdentity, Recipient},
        nodes::NodeIdentity,
    },
    anonymous_replies::requests::AnonymousSenderTag,
    receiver::ReconstructedMessage,
};
pub use nym_task::connections::TransmissionLane;
pub use nym_topology::{provider_trait::TopologyProvider, NymTopology};
pub use paths::StoragePaths;
pub use socks5_client::Socks5MixnetClient;
pub use traits::MixnetMessageSender;
