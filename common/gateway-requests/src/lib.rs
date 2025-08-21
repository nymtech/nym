// Copyright 2020-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_crypto::generic_array;
use nym_crypto::OutputSizeUser;
use nym_sphinx::params::GatewayIntegrityHmacAlgorithm;

pub use types::*;

pub mod authentication;
pub mod models;
pub mod registration;
pub mod shared_key;
pub mod types;

pub use shared_key::helpers::SymmetricKey;
pub use shared_key::legacy::{LegacySharedKeySize, LegacySharedKeys};
pub use shared_key::{
    SharedGatewayKey, SharedKeyConversionError, SharedKeyUsageError, SharedSymmetricKey,
};

pub type GatewayProtocolVersion = u8;

pub const CURRENT_PROTOCOL_VERSION: GatewayProtocolVersion = UPGRADE_MODE_VERSION;

/// Defines the current version of the communication protocol between gateway and clients.
/// It has to be incremented for any breaking change.
// history:
// 1 - initial release
// 2 - changes to client credentials structure
// 3 - change to AES-GCM-SIV and non-zero IVs
// 4 - introduction of v2 authentication protocol to prevent reply attacks
// 5 - add key rotation information to the serialised mix packet
// 6 - support for 'upgrade mode'
pub const INITIAL_PROTOCOL_VERSION: GatewayProtocolVersion = 1;
pub const CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION: GatewayProtocolVersion = 2;
pub const AES_GCM_SIV_PROTOCOL_VERSION: GatewayProtocolVersion = 3;
pub const AUTHENTICATE_V2_PROTOCOL_VERSION: GatewayProtocolVersion = 4;
pub const EMBEDDED_KEY_ROTATION_INFO_VERSION: GatewayProtocolVersion = 5;
pub const UPGRADE_MODE_VERSION: GatewayProtocolVersion = 6;

// TODO: could using `Mac` trait here for OutputSize backfire?
// Should hmac itself be exposed, imported and used instead?
pub type LegacyGatewayMacSize = <GatewayIntegrityHmacAlgorithm as OutputSizeUser>::OutputSize;

pub trait GatewayProtocolVersionExt {
    const CURRENT: GatewayProtocolVersion = CURRENT_PROTOCOL_VERSION;

    fn supports_aes256_gcm_siv(&self) -> bool;
    fn supports_authenticate_v2(&self) -> bool;
    fn supports_key_rotation_packet(&self) -> bool;
    fn supports_upgrade_mode(&self) -> bool;
    fn is_future_version(&self) -> bool;
}

impl GatewayProtocolVersionExt for Option<GatewayProtocolVersion> {
    fn supports_aes256_gcm_siv(&self) -> bool {
        let Some(protocol) = self else { return false };
        protocol.supports_aes256_gcm_siv()
    }

    fn supports_authenticate_v2(&self) -> bool {
        let Some(protocol) = self else { return false };
        protocol.supports_authenticate_v2()
    }

    fn supports_key_rotation_packet(&self) -> bool {
        let Some(protocol) = self else { return false };
        protocol.supports_key_rotation_packet()
    }

    fn supports_upgrade_mode(&self) -> bool {
        let Some(protocol) = self else { return false };
        protocol.supports_upgrade_mode()
    }

    fn is_future_version(&self) -> bool {
        let Some(protocol) = self else { return false };
        protocol.is_future_version()
    }
}

impl GatewayProtocolVersionExt for GatewayProtocolVersion {
    fn supports_aes256_gcm_siv(&self) -> bool {
        *self >= AES_GCM_SIV_PROTOCOL_VERSION
    }

    fn supports_authenticate_v2(&self) -> bool {
        *self >= AUTHENTICATE_V2_PROTOCOL_VERSION
    }

    fn supports_key_rotation_packet(&self) -> bool {
        *self >= EMBEDDED_KEY_ROTATION_INFO_VERSION
    }

    fn supports_upgrade_mode(&self) -> bool {
        *self >= UPGRADE_MODE_VERSION
    }

    fn is_future_version(&self) -> bool {
        *self > CURRENT_PROTOCOL_VERSION
    }
}
