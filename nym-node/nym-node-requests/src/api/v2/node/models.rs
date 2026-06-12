// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::node::models::AnnouncePorts;
use celes::Country;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Auxiliary details of the associated Nym Node.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AuxiliaryDetailsV2 {
    /// Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
    #[cfg_attr(feature = "openapi", schema(example = "PL", value_type = Option<String>))]
    #[schemars(with = "Option<String>")]
    #[schemars(length(equal = 2))]
    pub location: Option<Country>,

    /// On-chain address of this node
    pub address: String,

    #[serde(default)]
    pub announce_ports: AnnouncePorts,

    /// Specifies whether this node operator has agreed to the terms and conditions
    /// as defined at <https://nymtech.net/terms-and-conditions/operators/v1.0.0>
    // make sure to include the default deserialisation as this field hasn't existed when the struct was first created
    #[serde(default)]
    pub accepted_operator_terms_and_conditions: bool,
}
