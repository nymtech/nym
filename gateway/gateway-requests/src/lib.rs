// Copyright 2020-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crypto::generic_array;
use crypto::hmac::HmacOutput;
use crypto::OutputSizeUser;
use nymsphinx::params::GatewayIntegrityHmacAlgorithm;
pub use types::*;

pub mod authentication;
pub mod iv;
pub mod registration;
pub mod types;

/// Defines the current version of the communication protocol between gateway and clients.
/// It has to be incremented for any breaking change.
pub const PROTOCOL_VERSION: u8 = 1;

pub type GatewayMac = HmacOutput<GatewayIntegrityHmacAlgorithm>;

// TODO: could using `Mac` trait here for OutputSize backfire?
// Should hmac itself be exposed, imported and used instead?
pub type GatewayMacSize = <GatewayIntegrityHmacAlgorithm as OutputSizeUser>::OutputSize;
