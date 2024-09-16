// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod registration;
pub mod request;
pub mod response;

pub use registration::{ClientMac, ClientMessage, GatewayClient, InitMessage, Nonce};

#[cfg(feature = "verify")]
pub use registration::HmacSha256;

const VERSION: u8 = 1;
