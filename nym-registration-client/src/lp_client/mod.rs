// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! dVPN registration over the Lewes Protocol.
//!
//! The channel itself - connecting, handshaking, carrying frames, and telescoping through an entry
//! gateway - belongs to [`nym_lp_gateway_client`]. What lives here is what a client *says* over
//! that channel: a registration client borrows a channel and registers over it.
//!
//! - [`LpDvpnRegistrationClient`] registers with the gateway an
//!   [`LpGatewayClient`](nym_lp_gateway_client::LpGatewayClient) is connected to.
//! - [`NestedLpDvpnRegistrationClient`] registers with an exit gateway through an entry one.
//!
//! # Usage
//!
//! ```ignore
//! use nym_lp_gateway_client::LpGatewayClient;
//! use nym_registration_client::LpDvpnRegistrationClient;
//!
//! let mut client = LpGatewayClient::new_with_default_config(
//!     keypair,
//!     gateway_peer,
//!     gateway_lp_address,
//!     ciphersuite,
//!     gateway_lp_protocol,
//! );
//!
//! client.perform_handshake().await?;
//!
//! // the registration client borrows the channel, so the gateway client is still yours afterwards
//! let gateway_data = LpDvpnRegistrationClient::new(&mut client)
//!     .register(&mut rng, ...)
//!     .await?;
//! ```

mod bandwidth_claim;
mod dvpn;
pub(crate) mod helpers;

pub use dvpn::{LpDvpnRegistrationClient, NestedLpDvpnRegistrationClient};
