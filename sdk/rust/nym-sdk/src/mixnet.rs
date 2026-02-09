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
mod sink;
mod socks5_client;
pub mod stream;
mod traits;

// Local module exports
pub use client::{DisconnectedMixnetClient, IncludedSurbs, MixnetClientBuilder};
pub use config::Config;
pub use native_client::MixnetClient;
pub use native_client::MixnetClientSender;
pub use paths::StoragePaths;
pub use sink::{MixnetMessageSink, MixnetMessageSinkTranslator};
pub use socks5_client::Socks5MixnetClient;
pub use stream::{MixnetListener, MixnetStream, StreamId};
pub use traits::MixnetMessageSender;

// Re-exports from nym-client-core with documentation
#[allow(deprecated)]
pub use nym_client_core::client::base_client::storage::gateways_storage::GatewaysDetailsStore;

/// Information about a currently active gateway connection.
#[doc(alias = "Gateway")]
pub use nym_client_core::client::base_client::storage::gateways_storage::ActiveGateway;

/// Information about a gateway that failed to connect or is invalid.
pub use nym_client_core::client::base_client::storage::gateways_storage::BadGateway;

/// Registration details for a gateway including keys and connection info.
pub use nym_client_core::client::base_client::storage::gateways_storage::GatewayRegistration;

/// Ephemeral (in-memory) storage backend. Data is lost when the client disconnects.
pub use nym_client_core::client::base_client::storage::Ephemeral;

/// Trait for mixnet client storage implementations.
pub use nym_client_core::client::base_client::storage::MixnetClientStorage;

/// On-disk persistent storage backend. Data survives client restarts.
pub use nym_client_core::client::base_client::storage::OnDiskPersistent;

/// Receiver for client lifecycle events.
pub use nym_client_core::client::base_client::EventReceiver;

/// Sender for client lifecycle events.
pub use nym_client_core::client::base_client::EventSender;

/// Events emitted by the mixnet client during its lifecycle.
pub use nym_client_core::client::base_client::MixnetClientEvent;

/// A message to be sent through the mixnet.
pub use nym_client_core::client::inbound_messages::InputMessage;

/// In-memory ephemeral key storage. Keys are lost when the client disconnects.
pub use nym_client_core::client::key_manager::persistence::InMemEphemeralKeys;

/// Trait for key storage implementations.
pub use nym_client_core::client::key_manager::persistence::KeyStore;

/// On-disk key storage. Keys persist across client restarts.
pub use nym_client_core::client::key_manager::persistence::OnDiskKeys;

/// The client's cryptographic keys (identity, encryption, gateway shared key).
pub use nym_client_core::client::key_manager::ClientKeys;

/// Events related to mix traffic (packet sending/receiving).
pub use nym_client_core::client::mix_traffic::MixTrafficEvent;

/// File-system backed reply SURB storage.
pub use nym_client_core::client::replies::reply_storage::fs_backend::Backend as ReplyStorage;

/// Combined reply storage supporting multiple backends.
pub use nym_client_core::client::replies::reply_storage::CombinedReplyStorage;

/// Empty reply storage that discards all SURBs. Replies will not work.
pub use nym_client_core::client::replies::reply_storage::Empty as EmptyReplyStorage;

/// Trait for reply SURB storage implementations.
pub use nym_client_core::client::replies::reply_storage::ReplyStorageBackend;

// Re-exports from nym-credential-storage
/// Ephemeral (in-memory) credential storage. Credentials are lost on disconnect.
pub use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;

/// A ticketbook stored in the credential storage.
pub use nym_credential_storage::models::StoredIssuedTicketbook;

/// Trait for credential storage implementations.
pub use nym_credential_storage::storage::Storage as CredentialStorage;

// Re-exports from nym-crypto
/// Ed25519 digital signature cryptography (signing and verification).
pub use nym_crypto::asymmetric::ed25519;

/// X25519 elliptic curve Diffie-Hellman key exchange.
pub use nym_crypto::asymmetric::x25519;

// Re-exports from nym-network-defaults
/// Network configuration details (API endpoints, contract addresses, etc.).
pub use nym_network_defaults::NymNetworkDetails;

// Re-exports from nym-socks5-client-core
/// SOCKS5 proxy configuration.
pub use nym_socks5_client_core::config::Socks5;

// Re-exports from nym-sphinx
/// The Ed25519 public key identifying a client.
pub use nym_sphinx::addressing::clients::ClientIdentity;

/// A Nym network address for sending messages. Format: `identity.encryption@gateway`.
#[doc(alias = "Address")]
#[doc(alias = "NymAddress")]
pub use nym_sphinx::addressing::clients::Recipient;

/// Error when parsing a [`Recipient`] from a string.
pub use nym_sphinx::addressing::clients::RecipientFormattingError;

/// The Ed25519 public key identifying a mix node or gateway.
pub use nym_sphinx::addressing::nodes::NodeIdentity;

/// A tag identifying an anonymous sender, used for sending replies via SURBs.
pub use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;

/// A message reconstructed from Sphinx packets after traversing the mixnet.
pub use nym_sphinx::receiver::ReconstructedMessage;

// Re-exports from nym-statistics-common
/// Events related to connection statistics.
pub use nym_statistics_common::clients::connection::ConnectionStatsEvent;

/// Statistics events that can be reported by clients.
pub use nym_statistics_common::clients::ClientStatsEvents;

/// Channel for sending statistics events to be reported.
pub use nym_statistics_common::clients::ClientStatsSender;

// Re-exports from nym-task
/// Queue lengths for different transmission lanes, useful for backpressure.
pub use nym_task::connections::LaneQueueLengths;

/// Transmission lane for prioritizing different types of traffic.
pub use nym_task::connections::TransmissionLane;

// Re-exports from nym-topology
/// Trait for providing network topology information.
pub use nym_topology::provider_trait::TopologyProvider;

/// The network topology containing mix nodes, gateways, and their routing info.
pub use nym_topology::NymTopology;
