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
//! let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
//! let peer: mixnet::Recipient = "peer_nym_address...".parse().unwrap();
//!
//! // Open a stream -- returns AsyncRead + AsyncWrite
//! let mut stream = client.open_stream(peer, None).await.unwrap();
//! stream.write_all(b"hello via stream").await.unwrap();
//!
//! let mut buf = vec![0u8; 1024];
//! let n = stream.read(&mut buf).await.unwrap();
//!
//! client.disconnect().await;
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
