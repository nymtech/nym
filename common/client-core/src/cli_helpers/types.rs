// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct GatewayInfo {
    pub registration: OffsetDateTime,
    pub identity: identity::PublicKey,
    pub active: bool,

    pub typ: String,
    pub endpoint: Option<Url>,
    pub wg_tun_address: Option<Url>,
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

        if let Some(wg_tun_address) = &self.wg_tun_address {
            write!(f, " wg tun address: {wg_tun_address}")?;
        }
        Ok(())
    }
}
