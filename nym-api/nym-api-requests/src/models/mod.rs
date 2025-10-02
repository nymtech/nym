// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![allow(deprecated)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

pub mod api_status;
pub mod circulating_supply;
pub mod described;
pub mod legacy;
pub mod mixnet;
pub mod network;
pub mod network_monitor;
pub mod node_status;
pub mod schema_helpers;

// don't break existing imports
pub use api_status::*;
pub use circulating_supply::*;
pub use described::*;
pub use legacy::*;
pub use mixnet::*;
pub use network::*;
pub use network_monitor::*;
pub use node_status::*;
pub use schema_helpers::*;

pub use nym_mixnet_contract_common::{EpochId, KeyRotationId, KeyRotationState};
pub use nym_node_requests::api::v1::node::models::BinaryBuildInformationOwned;
pub use nym_noise_keys::VersionedNoiseKey;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct RequestError {
    message: String,
}

impl RequestError {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        RequestError {
            message: msg.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn empty() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl Display for RequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.message, f)
    }
}
