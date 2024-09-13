// Copyright 2020-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_crypto::generic_array;
use nym_crypto::hmac::HmacOutput;
use nym_crypto::OutputSizeUser;
use nym_sphinx::params::GatewayIntegrityHmacAlgorithm;
pub use types::*;

pub mod authentication;
pub mod iv;
pub mod models;
pub mod registration;
pub mod types;

pub const CURRENT_PROTOCOL_VERSION: u8 = AES_GCM_SIV_PROTOCOL_VERSION;

/// Defines the current version of the communication protocol between gateway and clients.
/// It has to be incremented for any breaking change.
// history:
// 1 - initial release
// 2 - changes to client credentials structure
// 3 - change to AES-GCM-SIV and non-zero IVs
pub const INITIAL_PROTOCOL_VERSION: u8 = 1;
pub const CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION: u8 = 2;
pub const AES_GCM_SIV_PROTOCOL_VERSION: u8 = 3;

pub type GatewayMac = HmacOutput<GatewayIntegrityHmacAlgorithm>;

// TODO: could using `Mac` trait here for OutputSize backfire?
// Should hmac itself be exposed, imported and used instead?
pub type GatewayMacSize = <GatewayIntegrityHmacAlgorithm as OutputSizeUser>::OutputSize;
