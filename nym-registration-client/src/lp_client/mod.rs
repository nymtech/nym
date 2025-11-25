// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) client implementation for direct gateway registration.
//!
//! This module provides a client for registering with gateways using the Lewes Protocol,
//! which offers direct TCP connections for improved performance compared to mixnet-based
//! registration while maintaining security through Noise protocol handshakes and credential
//! verification.
//!
//! # Usage
//!
//! ```ignore
//! use nym_registration_client::lp_client::LpRegistrationClient;
//!
//! let client = LpRegistrationClient::new_with_default_psk(
//!     keypair,
//!     gateway_public_key,
//!     gateway_lp_address,
//!     client_ip,
//! );
//!
//! // Establish TCP connection
//! client.connect().await?;
//!
//! // Perform handshake (nym-79)
//! client.perform_handshake().await?;
//!
//! // Register with gateway (nym-80, nym-81)
//! let response = client.register(credential, ticket_type).await?;
//! ```

mod client;
mod config;
mod error;
mod nested_session;
mod transport;

pub use client::LpRegistrationClient;
pub use config::LpConfig;
pub use error::LpClientError;
pub use nested_session::NestedLpSession;
