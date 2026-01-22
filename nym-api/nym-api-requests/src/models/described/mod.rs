// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_noise_keys::VersionedNoiseKeyV1;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use utoipa::ToSchema;

pub mod type_translation;
pub mod v1;
pub mod v2;

// don't break existing imports
pub use type_translation::*;
pub use v1::*;
pub use v2::*;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NoiseDetails {
    pub key: VersionedNoiseKeyV1,

    pub mixnet_port: u16,

    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,
}
