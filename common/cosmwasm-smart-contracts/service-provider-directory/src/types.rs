// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use nym_contracts_common::IdentityKey;
use std::fmt::{Display, Formatter};

/// The directory of services are indexed by [`ServiceId`].
pub type ServiceId = u32;

#[cw_serde]
pub struct Service {
    /// Unique id assigned to the anounced service.
    pub service_id: ServiceId,
    /// The announced service.
    pub service: ServiceDetails,
    /// Address of the service owner.
    pub announcer: Addr,
    /// Block height at which the service was added.
    pub block_height: u64,
    /// The deposit used to announce the service.
    pub deposit: Coin,
}

#[cw_serde]
pub struct ServiceDetails {
    /// The address of the service.
    pub nym_address: NymAddress,
    /// The service type.
    pub service_type: ServiceType,
    /// The identity key of the service.
    pub identity_key: IdentityKey,
}

/// The types of addresses supported.
#[cw_serde]
pub enum NymAddress {
    /// String representation of a nym address, which is of the form
    /// client_id.client_enc@gateway_id.
    Address(String),
    // String name that can looked up in the nym-name-service contract (once it exists)
    //Name(String),
}

impl NymAddress {
    /// Create a new nym address.
    pub fn new(address: &str) -> Self {
        Self::Address(address.to_string())
    }

    pub fn as_str(&self) -> &str {
        match self {
            NymAddress::Address(address) => address,
            //NymAddress::Name(name) => name,
        }
    }
}

impl Display for NymAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The type of services provider supported
#[cw_serde]
pub enum ServiceType {
    NetworkRequester,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let service_type = match self {
            ServiceType::NetworkRequester => "network_requester",
        };
        write!(f, "{service_type}")
    }
}
