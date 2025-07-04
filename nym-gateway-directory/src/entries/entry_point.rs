// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt::{Display, Formatter};

pub use nym_sdk::mixnet::NodeIdentity;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::gateway::{Gateway, GatewayList};
use crate::{Error, error::Result};

// The entry point is always a gateway identity, or some other entry that can be resolved to a
// gateway identity.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum EntryPoint {
    // An explicit entry gateway identity.
    Gateway { identity: NodeIdentity },
    // Select a random entry gateway in a specific location.
    Location { location: String },
    // Select an entry gateway at random.
    Random,
}

impl Display for EntryPoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryPoint::Gateway { identity } => write!(f, "Gateway: {identity}"),
            EntryPoint::Location { location } => write!(f, "Location: {location}"),
            EntryPoint::Random => write!(f, "Random"),
        }
    }
}

impl EntryPoint {
    pub fn from_base58_string(base58: &str) -> Result<Self> {
        let identity = NodeIdentity::from_base58_string(base58).map_err(|source| {
            Error::NodeIdentityFormattingError {
                identity: base58.to_string(),
                source,
            }
        })?;
        Ok(EntryPoint::Gateway { identity })
    }

    pub fn is_location(&self) -> bool {
        matches!(self, EntryPoint::Location { .. })
    }

    pub async fn lookup_gateway(&self, gateways: &GatewayList) -> Result<Gateway> {
        match &self {
            EntryPoint::Gateway { identity } => {
                debug!("Selecting gateway by identity: {}", identity);
                gateways
                    .gateway_with_identity(identity)
                    .ok_or_else(|| Error::NoMatchingGateway {
                        requested_identity: identity.to_string(),
                    })
                    .cloned()
            }
            EntryPoint::Location { location } => {
                debug!("Selecting gateway by location: {}", location);
                gateways
                    .random_gateway_located_at(location.to_string())
                    .ok_or_else(|| Error::NoMatchingEntryGatewayForLocation {
                        requested_location: location.clone(),
                        available_countries: gateways.all_iso_codes(),
                    })
            }
            EntryPoint::Random => {
                debug!("Selecting a random gateway");
                gateways
                    .random_gateway()
                    .ok_or_else(|| Error::FailedToSelectGatewayRandomly)
            }
        }
    }
}
