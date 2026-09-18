// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! How a client talks the Lewes Protocol to a gateway.
//!
//! This crate owns the *channel*: opening the connection, completing the KKT/PSQ handshake, and
//! carrying [`LpFrame`](nym_lp_data::packet::LpFrame)s over the resulting session. It does not know
//! what those frames say - registration, and anything else a client might send, live in the crates
//! that build on this one.
//!
//! Two shapes of channel:
//!
//! - [`LpGatewayClient`] talks to one gateway over its own connection.
//! - [`NestedLpSession`] telescopes: it handshakes with a second gateway *through* an established
//!   [`LpGatewayClient`], so the inner gateway sees the outer one's address rather than the
//!   client's. [`NestedConnection`] is what makes that possible - it implements the same channel
//!   traits as a TCP stream, so the handshake code cannot tell the difference.
//!
//! Both are generic over the channel type, so anything implementing
//! [`LpTransportChannel`](nym_lp::transport::traits::LpTransportChannel) and
//! [`LpHandshakeChannel`](nym_lp::transport::LpHandshakeChannel) works - a `TcpStream` in
//! production, an in-memory pair under test.

pub use client::LpGatewayClient;
pub use config::LpGatewayClientConfig;
pub use error::{LpClientError, Result};
pub use helpers::exponential_backoff_with_jitter;
pub use nested_session::{NestedLpSession, connection::NestedConnection};
pub use session_helpers::{extract_forwarded_response, prepare_send_packet};

mod client;
mod config;
mod error;
mod helpers;
mod nested_session;
mod session_helpers;
