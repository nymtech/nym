// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;
pub mod public_key;
pub mod registration;

pub use error::Error;
pub use public_key::PeerPublicKey;
pub use registration::{
    ClientMac, ClientMessage, ClientRegistrationResponse, GatewayClient, InitMessage, Nonce,
};

#[cfg(feature = "verify")]
pub use registration::HmacSha256;

pub const WG_PORT: u16 = 51822;
