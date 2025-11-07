// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! High-level streaming interface for the mixnet.
//!
//! # Basic Usage
//! ## Simple Send/Receive
//!
//! ```no_run
//! use nym_sdk::stream_wrapper::{MixSocket, MixStream, NetworkEnvironment};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let env = NetworkEnvironment::Mainnet;
//!
//!     // Create listener (no peer)
//!     let listener_socket = MixSocket::new(env.env_file_path()).await?;
//!     let listener_address = *listener_socket.local_addr();
//!     let mut listener_stream = listener_socket.into_stream();
//!
//!     // Create sender connected to listener
//!     let mut sender_stream = MixStream::connect(listener_address, env.env_file_path()).await?;
//!
//!     // Sender initiates
//!     sender_stream.send(b"Hello, Mixnet!").await?;
//!
//!     // Listener receives and extracts SURB tag
//!     let msg = listener_stream.recv().await?;
//!     assert_eq!(msg.message, b"Hello, Mixnet!");
//!
//!     // Store SURB and reply anonymously
//!     if let Some(surbs) = msg.sender_tag {
//!         listener_stream.store_surb_tag(surbs);
//!         listener_stream.send(b"Hello back!").await?;
//!     }
//!
//!     // Sender receives anonymous reply
//!     let reply = sender_stream.recv().await?;
//!     assert_eq!(reply.message, b"Hello back!");
//!
//!     Ok(())
//! }
//! ```
//!

mod mixnet_stream_wrapper;
mod mixnet_stream_wrapper_ipr;
mod network_env;

pub use mixnet_stream_wrapper::{MixSocket, MixStream, MixStreamReader, MixStreamWriter};
pub use mixnet_stream_wrapper_ipr::{IpMixStream, IpMixStreamReader, IpMixStreamWriter};
pub use network_env::NetworkEnvironment;
