// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) client implementation for direct gateway registration.
//!
//! This module provides a client for registering with gateways using the Lewes Protocol,
//! which offers direct TCP connections for improved performance compared to mixnet-based
//! registration while maintaining security through Noise protocol handshakes and credential
//! verification.
//!
//! Uses a packet-per-connection model: each LP packet exchange opens a new TCP connection,
//! sends one packet, receives one response, then closes. Session state is maintained in
//! the state machine across connections.
//!
//! # Usage
//!
//! ```ignore
//! use nym_registration_client::lp_client::LpRegistrationClient;
//!
//! let mut client = LpRegistrationClient::new_with_default_psk(
//!     keypair,
//!     gateway_public_key,
//!     gateway_lp_address,
//!     client_ip,
//! );
//!
//! // Perform handshake (multiple packet-per-connection exchanges)
//! client.perform_handshake().await?;
//!
//! // Register with gateway (single packet-per-connection exchange)
//! let gateway_data = client.register(wg_keypair, gateway_identity, bandwidth_controller, ticket_type).await?;
//! ```

mod client;
mod config;
mod error;
mod nested_session;

pub use client::LpRegistrationClient;
pub use config::LpConfig;
pub use error::LpClientError;
pub use nested_session::NestedLpSession;
