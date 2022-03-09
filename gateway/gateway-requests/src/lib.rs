// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
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

pub type GatewayMac = HmacOutput<GatewayIntegrityHmacAlgorithm>;

// TODO: could using `Mac` trait here for OutputSize backfire?
// Should hmac itself be exposed, imported and used instead?
pub type GatewayMacSize = <GatewayIntegrityHmacAlgorithm as OutputSizeUser>::OutputSize;
