use std::fmt::{Display, Formatter};

use cosmwasm_std::{Addr, Coin};
use serde::{Deserialize, Serialize};

/// The directory of services are indexed by [`ServiceId`].
pub type ServiceId = u32;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Service {
    /// The address of the service.
    pub nym_address: NymAddress,
    /// The service type.
    pub service_type: ServiceType,
    /// Service owner.
    pub announcer: Addr,
    /// Block height at which the service was added.
    pub block_height: u64,
    /// The deposit used to announce the service.
    pub deposit: Coin,
}

/// The types of addresses supported.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum NymAddress {
    /// String representation of a nym address, which is of the form
    /// client_id.client_enc@gateway_id.
    Address(String),
    // For the future when we have a nym-dns contract
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
        }
    }
}

impl Display for NymAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The type of services provider supported
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
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
