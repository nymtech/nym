// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! How a client reaches the gateways it talks the Lewes Protocol to.
//!
//! This crate is the *transport*: it opens connections, runs the KKT/PSQ handshake, and moves
//! packets. It does not encrypt them and it does not hold sessions - a handshake hands its
//! [`LpTransportSession`](nym_lp::LpTransportSession) to whoever asked for it, and everything that
//! turns a message into ciphertext lives in the crates that build on this one.
//!
//! [`LpGatewayClient`] covers both planes, because they are shaped differently:
//!
//! - **Control** is a stream ([`LpTransportChannel`](nym_lp::transport::traits::LpTransportChannel)),
//!   one connection per gateway, request/response. The handshake needs ordering and delivery, and
//!   so does anything expecting an answer.
//! - **Data** is a single datagram socket
//!   ([`LpDatagramChannel`](nym_lp::transport::traits::LpDatagramChannel)) shared by every gateway.
//!   Frames go out and replies arrive out of band.
//!
//! [`NestedLpSession`] telescopes: it handshakes with a second gateway *through* an established
//! control connection, so the inner gateway sees the outer one's address rather than the client's.
//! [`NestedConnection`] is what makes that possible - it implements the same channel traits as a
//! TCP stream, so the handshake code cannot tell the difference.
//!
//! Both sides are generic over their channel type, defaulting to `TcpStream` and `UdpSocket`, so a
//! caller that only registers can ignore the data half entirely and a test can swap in an
//! in-memory pair.

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
