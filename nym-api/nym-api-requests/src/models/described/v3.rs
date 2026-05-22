// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::described::v2::{AnnouncePortsV2, NymNodeAuxiliaryDetailsV2};
use celes::Country;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type AnnouncePortsV3 = AnnouncePortsV2;

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq,
)]
pub struct NymNodeAuxiliaryDetailsV3 {
    /// Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
    #[schema(example = "PL", value_type = Option<String>)]
    #[schemars(with = "Option<String>")]
    #[schemars(length(equal = 2))]
    pub location: Option<Country>,

    /// On-chain address of this node
    pub address: Option<String>,

    #[serde(default)]
    pub announce_ports: AnnouncePortsV3,

    /// Specifies whether this node operator has agreed to the terms and conditions
    /// as defined at <https://nymtech.net/terms-and-conditions/operators/v1.0.0>
    // make sure to include the default deserialisation as this field hasn't existed when the struct was first created
    #[serde(default)]
    pub accepted_operator_terms_and_conditions: bool,
}

impl From<NymNodeAuxiliaryDetailsV3> for NymNodeAuxiliaryDetailsV2 {
    fn from(value: NymNodeAuxiliaryDetailsV3) -> Self {
        NymNodeAuxiliaryDetailsV2 {
            location: value.location,
            announce_ports: value.announce_ports,
            accepted_operator_terms_and_conditions: value.accepted_operator_terms_and_conditions,
        }
    }
}
