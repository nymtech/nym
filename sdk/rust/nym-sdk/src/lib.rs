// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # Nym SDK
//!
//! Rust SDK for building privacy-preserving applications on the [Nym mixnet](https://nymtech.net),
//! a decentralized network that provides network-level privacy through packet mixing,
//! timing obfuscation, and Sphinx packet encryption.
//!
//! For tutorials and conceptual guides, see the
//! [Nym developer portal](https://nymtech.net/docs/developers/rust).
//!
//! # Getting started
//!
//! **Start with [`mixnet::MixnetClient::connect_new`]** for a quick ephemeral client, or
//! [`mixnet::MixnetClientBuilder`] when you need to configure storage, gateway selection,
//! or network settings.
//!
//! ```no_run
//! use nym_sdk::mixnet::{self, MixnetMessageSender};
//!
//! # #[tokio::main]
//! # async fn main() {
//! let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
//! let addr = *client.nym_address();
//!
//! client.send_plain_message(addr, "hello mixnet!").await.unwrap();
//!
//! // Always disconnect for clean shutdown
//! client.disconnect().await;
//! # }
//! ```
//!
//! ## Stream I/O
//!
//! For persistent bidirectional byte channels (like a TCP socket), use
//! [`MixnetClient::open_stream`](mixnet::MixnetClient::open_stream) and
//! [`MixnetClient::listener`](mixnet::MixnetClient::listener).
//! Streams implement [`AsyncRead`](tokio::io::AsyncRead) +
//! [`AsyncWrite`](tokio::io::AsyncWrite) — see [`mixnet::stream`] for
//! the full API:
//!
//! ```no_run
//! use nym_sdk::mixnet;
//! use tokio::io::{AsyncReadExt, AsyncWriteExt};
//!
//! # #[tokio::main]
//! # async fn main() {
//! let mut sender = mixnet::MixnetClient::connect_new().await.unwrap();
//! let mut receiver = mixnet::MixnetClient::connect_new().await.unwrap();
//! let recv_addr = *receiver.nym_address();
//!
//! let mut listener = receiver.listener().unwrap();
//! let mut tx = sender.open_stream(recv_addr, None).await.unwrap();
//! let mut rx = listener.accept().await.unwrap();
//!
//! tx.write_all(b"hello via stream").await.unwrap();
//! tx.flush().await.unwrap();
//!
//! let mut buf = vec![0u8; 1024];
//! let n = rx.read(&mut buf).await.unwrap();
//!
//! sender.disconnect().await;
//! receiver.disconnect().await;
//! # }
//! ```
//!
//! See [`mixnet::stream`] for the full stream API, and the
//! [stream tutorial](https://nymtech.net/docs/developers/rust/stream/tutorial)
//! for a step-by-step walkthrough.
//!
//! # Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`mixnet`] | Core client — messages, streams, builder, storage |
//! | [`client_pool`] | Pre-warmed pool of ephemeral clients |
//! | [`tcp_proxy`] | TCP tunnelling over the mixnet (deprecated — prefer streams) |
//! | [`bandwidth`] | Bandwidth credential management |
//!
//! # Feature flags
//!
//! **Feature gates are not yet implemented.** Importing `nym-sdk` currently pulls in all
//! modules and their full dependency trees. Work is planned to gate modules behind Cargo
//! features so you can import only what you need.
//!
//! # Network configuration
//!
//! By default, the SDK connects to the Nym mainnet. Customize with
//! [`NymNetworkDetails`] or environment variables.

mod error;

pub mod bandwidth;
pub mod client_pool;
pub mod ip_packet_client;
pub mod ipr_wrapper;
pub mod mixnet;
pub mod tcp_proxy;

pub use error::{Error, Result};

// Re-exports: gateway transceiver types (deprecated internals)
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::{
    ErasedGatewayError, GatewayPacketRouter, GatewayReceiver, GatewaySender, GatewayTransceiver,
    LocalGateway, LocalGatewayError, MockGateway, MockGatewayError, PacketRouter, RemoteGateway,
};

// Re-exports: topology
/// Fetches network topology from the Nym API.
pub use nym_client_core::client::topology_control::NymApiTopologyProvider;
/// Configuration for [`NymApiTopologyProvider`].
pub use nym_client_core::client::topology_control::NymApiTopologyProviderConfig;
/// Trait for custom topology providers. Implement this to fetch topology
/// from alternative sources (see `custom_topology_provider` example).
pub use nym_client_core::client::topology_control::TopologyProvider;

// Re-exports: config
/// Debug/development configuration for mixnet clients.
pub use nym_client_core::config::DebugConfig;
/// Controls whether client identity persists across restarts.
pub use nym_client_core::config::RememberMe;

// Re-exports: network defaults
/// Cosmos chain configuration (chain ID, RPC, gas).
pub use nym_network_defaults::ChainDetails;
/// Token denomination details (borrowed).
pub use nym_network_defaults::DenomDetails;
/// Token denomination details (owned).
pub use nym_network_defaults::DenomDetailsOwned;
/// Nym smart contract addresses.
pub use nym_network_defaults::NymContracts;
/// Complete network configuration (endpoints, contracts, chain details).
///
/// ```rust,no_run
/// use nym_sdk::NymNetworkDetails;
///
/// // Load from environment (defaults to mainnet)
/// let network = NymNetworkDetails::new_from_env();
/// println!("API: {:?}", network.endpoints);
/// ```
pub use nym_network_defaults::NymNetworkDetails;
/// Validator/API endpoint configuration.
pub use nym_network_defaults::ValidatorDetails;

// Re-exports: task management
/// Cancellation token for graceful shutdown.
pub use nym_task::ShutdownToken;
/// Tracks spawned tasks for coordinated shutdown.
pub use nym_task::ShutdownTracker;

// Re-exports: API client
/// Client identification sent with Nym API requests.
pub use nym_validator_client::UserAgent;
