use std::fmt::{Display, Formatter};

use cosmwasm_std::{Addr, Coin};
//use nym_sphinx_addressing::clients::Recipient;
use serde::{Deserialize, Serialize};

/// The directory of services are indexed by [`ServiceId`].
pub type NameId = u32;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RegisteredName {
    /// The name pointing to the nym address
    pub name: NymName,
    /// The address of the service.
    pub nym_address: NymAddress,
    /// Service owner.
    pub owner: Addr,
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
    // WIP(JON): replace with struct
    Address(String),
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

/// Name stored and pointing a to a nym-address
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub struct NymName(pub String);

impl NymName {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for NymName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// [`RegisterdName`] together with the assigned [`NameId`].
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
// WIP(JON): consider renaming to NameEntry
pub struct NameInfo {
    pub name_id: NameId,
    pub name: RegisteredName,
}

impl NameInfo {
    pub fn new(name_id: NameId, name: RegisteredName) -> Self {
        Self { name_id, name }
    }
}
