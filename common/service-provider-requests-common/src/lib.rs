// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServiceProviderType {
    Authenticator,
    IpPacketRouter,
    NetworkRequester,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Protocol {
    pub version: u8,
    pub service_provider_type: ServiceProviderType,
}
