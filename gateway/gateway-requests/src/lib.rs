// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::hmac::{hmac::Mac, HmacOutput};
use nymsphinx::params::GatewayIntegrityHmacAlgorithm;

pub mod authentication;
pub mod registration;
pub mod types;

pub const DUMMY_MESSAGE_CONTENT: &[u8] =
    b"[DUMMY MESSAGE] Wanting something does not give you the right to have it.";

pub use crypto::generic_array;
pub use types::*;

pub type GatewayMac = HmacOutput<GatewayIntegrityHmacAlgorithm>;

// TODO: could using `Mac` trait here for OutputSize backfire?
// Should hmac itself be exposed, imported and used instead?
pub type GatewayMacSize = <GatewayIntegrityHmacAlgorithm as Mac>::OutputSize;
