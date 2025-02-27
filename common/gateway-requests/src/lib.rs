// Copyright 2020-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_crypto::generic_array;

pub use types::*;

pub mod models;
pub mod registration;
pub mod shared_key;
pub mod types;

pub use shared_key::{SharedKeyConversionError, SharedKeyUsageError, SharedSymmetricKey};

pub const CURRENT_PROTOCOL_VERSION: u8 = AUTHENTICATE_V2_PROTOCOL_VERSION;

/// Defines the current version of the communication protocol between gateway and clients.
/// It has to be incremented for any breaking change.
// history:
// 1 - initial release
// 2 - changes to client credentials structure
// 3 - change to AES-GCM-SIV and non-zero IVs
// 4 - introduction of v2 authentication protocol to prevent reply attacks
pub const INITIAL_PROTOCOL_VERSION: u8 = 1;
pub const CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION: u8 = 2;
pub const AES_GCM_SIV_PROTOCOL_VERSION: u8 = 3;
pub const AUTHENTICATE_V2_PROTOCOL_VERSION: u8 = 4;

pub trait GatewayProtocolVersionExt {
    fn supports_aes256_gcm_siv(&self) -> bool;
    fn supports_authenticate_v2(&self) -> bool;
}

impl GatewayProtocolVersionExt for u8 {
    fn supports_aes256_gcm_siv(&self) -> bool {
        *self >= AES_GCM_SIV_PROTOCOL_VERSION
    }

    fn supports_authenticate_v2(&self) -> bool {
        *self >= AUTHENTICATE_V2_PROTOCOL_VERSION
    }
}
