// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(
    Display,
    Default,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    EnumString,
    PartialEq,
    Eq,
    JsonSchema,
    PartialOrd,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Load {
    #[default]
    Unknown,

    // order of the variants matter as we've derived `PartialOrd` on them
    Negligible, // 0 - 0.1
    Low,        // 0.1 - 0.3
    Medium,     // 0.3 - 0.6
    High,       // 0.6 - 0.8
    VeryHigh,   // 0.8 - 0.95
    AtCapacity, // >= 0.95
}

impl Load {
    // returns load of one tier higher
    pub fn increment(&self) -> Self {
        match self {
            Self::Unknown => Self::Unknown,
            Self::Negligible => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::VeryHigh,
            Self::VeryHigh => Self::AtCapacity,
            Self::AtCapacity => Self::AtCapacity,
        }
    }
}

impl From<f64> for Load {
    fn from(value: f64) -> Self {
        if value <= 0.1 {
            Self::Negligible
        } else if value <= 0.3 {
            Self::Low
        } else if value <= 0.6 {
            Self::Medium
        } else if value <= 0.8 {
            Self::High
        } else if value <= 0.95 {
            Self::VeryHigh
        } else {
            Self::AtCapacity
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodeLoad {
    pub total: Load,
    pub machine: Load,
    pub network: Load,
}
