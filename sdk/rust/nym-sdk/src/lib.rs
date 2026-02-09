// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # Nym SDK
//!
//! Rust SDK for building privacy-preserving applications on the Nym platform.
//!
//! ## Overview
//!
//! This crate provides high-level abstractions for interacting with the Nym mixnet,
//! a decentralized network that provides network-level privacy through mix node routing
//! and Sphinx packet encryption.
//!
//! ## Key Modules
//!
//! - [`mixnet`] - Core mixnet client functionality for sending and receiving messages
//!   through the Nym network. Use [`mixnet::MixnetClient`] for direct mixnet interaction
//!   or [`mixnet::MixnetClientBuilder`] to configure client options.
//!
//! - [`tcp_proxy`] - TCP proxy components for tunneling existing TCP traffic through the
//!   mixnet. Use [`tcp_proxy::NymProxyClient`] on the client side and
//!   [`tcp_proxy::NymProxyServer`] on the server side. This is the recommended starting
//!   point for integrating existing applications.
//!
//! - [`client_pool`] - A connection pool for managing multiple [`mixnet::MixnetClient`]
//!   instances. Useful for high-throughput applications that need to handle many
//!   concurrent connections.
//!
//! - [`bandwidth`] - Bandwidth credential management for importing ticketbooks and
//!   verification keys needed for paid network access.
//!
//! ## Quick Start
//!
//! ### Basic Mixnet Client
//!
//! ```rust,no_run
//! use nym_sdk::mixnet;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create an ephemeral client (keys discarded on shutdown)
//!     let mut client = mixnet::MixnetClient::connect_new().await?;
//!
//!     println!("Client address: {}", client.nym_address());
//!
//!     // Send a message to another Nym address
//!     // client.send_plain_message(recipient, "Hello!").await?;
//!
//!     client.disconnect().await;
//!     Ok(())
//! }
//! ```
//!
//! ### TCP Proxy
//!
//! For tunneling existing TCP applications, see the [`tcp_proxy`] module documentation
//! which provides detailed examples of client and server setup.
//!
//! ## Network Configuration
//!
//! By default, the SDK connects to the Nym mainnet. Network configuration can be
//! customized using [`NymNetworkDetails`] or by setting environment variables.
//!
//! ## Re-exports
//!
//! This crate re-exports commonly used types from underlying Nym crates for convenience:
//! - Network configuration types from `nym-network-defaults`
//! - Shutdown handling from `nym-task`
//! - Topology providers from `nym-client-core`
//! - Gateway communication traits and types from `nym-client-core`

mod error;

pub mod bandwidth;
pub mod client_pool;
pub mod ip_packet_client;
pub mod ipr_wrapper;
pub mod mixnet;
pub mod tcp_proxy;

pub use error::{Error, Result};

// Re-exports from nym-client-core: gateway transceiver types

/// Type-erased gateway error for dynamic dispatch.
///
/// Wraps gateway-specific errors to allow using different gateway implementations
/// (remote, local, mock) through trait objects.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::ErasedGatewayError;

/// Trait for receiving packets from the mixnet via a gateway.
///
/// Defines the functionality for correctly routing packets received from the mixnet,
/// distinguishing between acknowledgements and regular messages.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::GatewayReceiver;

/// Trait for sending mix packets into the mixnet.
///
/// Defines the functionality for sending Sphinx packets into the mixnet,
/// typically through a gateway connection.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::GatewaySender;

/// Combined trait for bidirectional gateway communication.
///
/// Combines [`GatewaySender`] and [`GatewayReceiver`] functionality for full
/// duplex communication with the mixnet through a gateway.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::GatewayTransceiver;

/// Gateway running within the same process.
///
/// Used when the client and gateway are co-located, avoiding network overhead.
/// Primarily used for embedded gateway scenarios.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::LocalGateway;

/// Errors specific to [`LocalGateway`] operations.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::LocalGatewayError;

/// Mock gateway implementation for testing.
///
/// A test double that records sent packets without actually transmitting them.
/// Useful for unit testing client code without network dependencies.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::MockGateway;

/// Error type for [`MockGateway`] operations.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::MockGatewayError;

/// Gateway connected via network socket (typically WebSocket).
///
/// The standard gateway connection type for clients connecting to remote
/// gateways over the network.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::RemoteGateway;

/// Routes packets to their appropriate handlers (ACKs vs regular messages).
///
/// This type is re-exported from `nym-gateway-client` and handles the routing
/// of incoming mixnet packets to the correct processing pipeline.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::GatewayPacketRouter;

/// Packet routing configuration for incoming mixnet traffic.
///
/// Configures how received packets are dispatched to acknowledgement handlers
/// versus message reconstruction pipelines.
#[allow(deprecated)]
pub use nym_client_core::client::mix_traffic::transceiver::PacketRouter;

// Re-exports from nym-client-core: topology providers
/// Provides network topology by querying the Nym API.
///
/// This is the standard topology provider that fetches mix node and gateway
/// information from the Nym validator API.
pub use nym_client_core::client::topology_control::NymApiTopologyProvider;

/// Configuration for [`NymApiTopologyProvider`].
///
/// Allows customizing how topology is fetched, including refresh intervals
/// and filtering options.
pub use nym_client_core::client::topology_control::NymApiTopologyProviderConfig;

/// Trait for providing network topology information to clients.
///
/// Implement this trait to create custom topology providers that fetch
/// network information from alternative sources.
pub use nym_client_core::client::topology_control::TopologyProvider;

// Re-exports from nym-client-core: config types
/// Debug configuration for mixnet clients.
///
/// Contains settings for debugging and development, such as traffic analysis
/// options and verbose logging.
pub use nym_client_core::config::DebugConfig;

/// Configuration for client identity persistence behavior.
///
/// Controls whether the client should remember its identity across restarts
/// or generate a new ephemeral identity each time.
pub use nym_client_core::config::RememberMe;

// Re-exports from nym-network-defaults
/// Blockchain network configuration details.
///
/// Contains information about the Cosmos chain used by Nym, including
/// chain ID, RPC endpoints, and gas configuration.
pub use nym_network_defaults::ChainDetails;

/// Token denomination details (borrowed).
///
/// Information about a token denomination used on the Nym network,
/// including display name and decimal precision.
pub use nym_network_defaults::DenomDetails;

/// Token denomination details (owned).
///
/// Owned version of [`DenomDetails`] for cases where the data needs
/// to outlive the source configuration.
pub use nym_network_defaults::DenomDetailsOwned;

/// Smart contract addresses for the Nym network.
///
/// Contains addresses of all Nym smart contracts deployed on the blockchain,
/// including the mixnet contract, vesting contract, and others.
pub use nym_network_defaults::NymContracts;

/// Complete Nym network configuration.
///
/// The primary configuration type containing all network endpoints, contract
/// addresses, and chain details. Can be loaded from environment variables
/// or constructed manually.
///
/// # Example
///
/// ```rust,no_run
/// use nym_sdk::NymNetworkDetails;
///
/// // Load from environment (defaults to mainnet)
/// let network = NymNetworkDetails::new_from_env();
///
/// // Get the API endpoint
/// println!("API: {:?}", network.endpoints);
/// ```
pub use nym_network_defaults::NymNetworkDetails;

/// Validator/API endpoint details.
///
/// Contains URLs and configuration for connecting to Nym validators
/// and API servers.
pub use nym_network_defaults::ValidatorDetails;

// Re-exports from nym-task
/// A cancellation token for signaling shutdown requests.
///
/// Wraps a [`tokio_util::sync::CancellationToken`] and is used for signaling
/// and listening for graceful shutdown requests across async tasks.
pub use nym_task::ShutdownToken;

/// Tracks spawned tasks and coordinates graceful shutdown.
///
/// Provides functionality to track nested tasks and coordinate their
/// shutdown without passing the entire shutdown manager around.
pub use nym_task::ShutdownTracker;

// Re-exports from nym-validator-client
/// Client identification sent to the Nym API.
///
/// Contains application name, version, and platform information sent
/// with API requests for analytics and debugging purposes.
pub use nym_validator_client::UserAgent;
