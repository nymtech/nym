// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::ed25519;
use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct GatewayInfo {
    pub registration: OffsetDateTime,
    pub identity: ed25519::PublicKey,
    pub active: bool,

    pub typ: String,
    pub endpoint: Option<Url>,
    pub fallback_endpoint: Option<Url>,
}

impl Display for GatewayInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.active {
            write!(f, "[ACTIVE] ")?;
        }
        write!(
            f,
            "{} gateway '{}' registered at: {}",
            self.typ, self.identity, self.registration
        )?;
        if let Some(endpoint) = &self.endpoint {
            write!(f, " endpoint: {endpoint}")?;
        }
        if let Some(fallback_endpoint) = &self.fallback_endpoint {
            write!(f, " fallback: {fallback_endpoint}")?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct RefreshedGatewayKeys {
    pub gateway: ed25519::PublicKey,
    pub address: Recipient,
}

impl Display for RefreshedGatewayKeys {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "registered with gateway '{}' under new keys, the client's address there is now {}",
            self.gateway, self.address
        )
    }
}
