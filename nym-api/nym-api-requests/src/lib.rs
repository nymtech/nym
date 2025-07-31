// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod constants;
pub mod ecash;
mod helpers;
pub mod legacy;
pub mod models;
pub mod nym_nodes;
pub mod pagination;
pub mod signable;

// The response type we fetch from the network details endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NymNetworkDetailsResponse {
    pub network: nym_config::defaults::NymNetworkDetails,
}

pub trait Deprecatable {
    fn deprecate(self) -> Deprecated<Self>
    where
        Self: Sized,
    {
        self.into()
    }
}

impl<T> Deprecatable for T {}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Deprecated<T> {
    pub deprecated: bool,
    #[serde(flatten)]
    pub response: T,
}

impl<T> From<T> for Deprecated<T> {
    fn from(response: T) -> Self {
        Deprecated {
            deprecated: true,
            response,
        }
    }
}
