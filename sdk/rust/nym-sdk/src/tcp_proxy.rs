//! TCP proxy functionality for routing socket connections through the Nym mixnet.
//!
//! # Deprecated
//!
//! <div class="warning">
//!
//! This module is deprecated. For new projects, use the
//! [`stream`](crate::mixnet::stream) module instead, which provides
//! `AsyncRead + AsyncWrite` streams directly over the mixnet without the
//! TCP socket overhead.
//!
//! </div>
//!
//! This module provides [`NymProxyClient`] and [`NymProxyServer`] for creating
//! TCP proxy tunnels that route traffic through the Nym mixnet for enhanced privacy.
//!
//! # Architecture
//!
//! The TCP proxy system consists of two components:
//!
//! - **[`NymProxyClient`]** - Listens for local TCP connections and forwards them
//!   through the mixnet to a remote `NymProxyServer`
//! - **[`NymProxyServer`]** - Receives connections from the mixnet and forwards
//!   them to a local upstream service
//!
//! ```text
//! ┌─────────────┐     ┌─────────────────┐     ┌─────────────┐     ┌─────────────────┐     ┌──────────────┐
//! │ Application │────▶│  NymProxyClient │────▶│   Mixnet    │────▶│  NymProxyServer │────▶│   Upstream   │
//! │  (Client)   │◀────│  (localhost)    │◀────│  (anonymity)│◀────│  (remote)       │◀────│   Service    │
//! └─────────────┘     └─────────────────┘     └─────────────┘     └─────────────────┘     └──────────────┘
//! ```
//!
//! # Message Ordering
//!
//! Since the mixnet does not guarantee message ordering, the proxy implements
//! a session-based ordering system using [`MessageBuffer`] and [`ProxiedMessage`].
//! Each message includes a session ID and sequence number for proper reassembly.
//!
//! # Example
//!
//! ## Client Side
//!
//! ```rust,no_run
//! use nym_sdk::tcp_proxy::NymProxyClient;
//! use nym_sphinx::addressing::Recipient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Parse the server's Nym address
//!     let server_address: Recipient = "server_nym_address...".parse()?;
//!
//!     // Create a proxy client listening on localhost:8080
//!     let client = NymProxyClient::new(
//!         server_address,
//!         "127.0.0.1",
//!         "8080",
//!         60,  // close timeout in seconds
//!         None, // use default network
//!         2,   // client pool size
//!     ).await?;
//!
//!     // Run the proxy (blocks until disconnected)
//!     client.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Server Side
//!
//! ```rust,no_run
//! use nym_sdk::tcp_proxy::NymProxyServer;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a proxy server that forwards to localhost:3000
//!     let mut server = NymProxyServer::new("127.0.0.1:3000", "./nym-proxy-data", None, None).await?;
//!
//!     println!("Server listening at: {}", server.nym_address());
//!
//!     // Run the server (blocks until disconnected)
//!     server.run_with_shutdown().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Utilities
//!
//! The [`utils`] submodule provides the message ordering infrastructure:
//!
//! - [`ProxiedMessage`] - A message with session ID and sequence number
//! - [`MessageBuffer`] - Orders out-of-order messages before delivery
//! - [`Payload`] - Message payload (data or close signal)
//! - [`DecayWrapper`] - Tracks message age for time-based delivery

mod tcp_proxy_client;
mod tcp_proxy_server;
pub mod utils;

pub use tcp_proxy_client::NymProxyClient;
pub use tcp_proxy_server::NymProxyServer;
pub use utils::{DecayWrapper, MessageBuffer, Payload, ProxiedMessage};
