// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use nym_contracts_common::IdentityKey;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};
use thiserror::Error;

use crate::error::{NameServiceError, Result};

/// The directory of names are indexed by [`NameId`].
pub type NameId = u32;

#[cw_serde]
pub struct RegisteredName {
    /// Unique id assigned to the registerd name.
    pub id: NameId,

    /// The registerd name details.
    pub name: NameDetails,

    /// name owner.
    pub owner: Addr,

    /// Block height at which the name was added.
    pub block_height: u64,

    /// The deposit used to announce the name.
    pub deposit: Coin,
}

impl RegisteredName {
    // Shortcut for getting the actual name
    pub fn entry(&self) -> &NymName {
        &self.name.name
    }
}

#[cw_serde]
pub struct NameDetails {
    /// The name pointing to the nym address
    pub name: NymName,

    /// The address of the name alias.
    pub address: Address,

    /// The identity key of the registered name.
    pub identity_key: IdentityKey,
}

/// String representation of a nym address, which is of the form
/// client_id.client_enc@gateway_id.
/// NOTE: entirely unvalidated.
#[cw_serde]
pub enum Address {
    NymAddress(NymAddressInner),
    // Possible extension:
    //Gateway(String)
}

#[cw_serde]
pub struct NymAddressInner {
    client_id: String,
    client_enc: String,
    gateway_id: String,
}

// ADDRESS . ENCRYPTION @ GATEWAY_ID
impl std::fmt::Display for NymAddressInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}@{}",
            self.client_id, self.client_enc, self.gateway_id
        )
    }
}

impl Address {
    /// Create a new nym address.
    pub fn new(address: &str) -> Result<Self> {
        parse_nym_address(address)
            .map(Self::NymAddress)
            .ok_or_else(|| NameServiceError::InvalidNymAddress(address.to_string()))
    }

    pub fn client_id(&self) -> &str {
        match self {
            Address::NymAddress(address) => &address.client_id,
        }
    }

    pub fn client_enc(&self) -> &str {
        match self {
            Address::NymAddress(address) => &address.client_enc,
        }
    }

    pub fn gateway_id(&self) -> &str {
        match self {
            Address::NymAddress(address) => &address.gateway_id,
        }
    }

    pub fn event_tag(&self) -> &str {
        match self {
            Address::NymAddress(_) => "nym_address",
            //Address::Gateway(_) => "gatway_address",
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::NymAddress(address) => write!(f, "{}", address),
        }
    }
}

// A valid nym address is of the form client_id.client_enc@gateway_id
fn parse_nym_address(address: &str) -> Option<NymAddressInner> {
    let parts: Vec<&str> = address.split('@').collect();
    if parts.len() != 2 {
        return None;
    }

    let client_part = parts[0];
    let gateway_part = parts[1];

    // The client part consists of two parts separated by a dot
    let client_parts: Vec<&str> = client_part.split('.').collect();
    if client_parts.len() != 2 {
        return None;
    }

    // Check that the gateway part does not contain any dots
    if gateway_part.contains('.') {
        return None;
    }

    Some(NymAddressInner {
        client_id: client_parts[0].to_string(),
        client_enc: client_parts[1].to_string(),
        gateway_id: gateway_part.to_string(),
    })
}

/// Name stored and pointing a to a nym-address
#[cw_serde]
pub struct NymName(String);

#[derive(Debug, Error)]
pub enum NymNameError {
    #[error("invalid name")]
    InvalidName,
}

/// Defines what names are allowed
fn is_valid_name_char(c: char) -> bool {
    // Normal lowercase letters
    (c.is_alphabetic() && c.is_lowercase())
        // or numbers
        || c.is_numeric()
        // special case hyphen or underscore
        || c == '-' || c == '_'
}

impl NymName {
    pub fn new(name: &str) -> Result<NymName, NymNameError> {
        // We are a bit restrictive in which names we allow, to start out with. Consider relaxing
        // this in the future.
        if !name.chars().all(is_valid_name_char) {
            return Err(NymNameError::InvalidName);
        }
        Ok(Self(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for NymName {
    type Err = NymNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Display for NymName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::NymName;

    #[test]
    fn parse_nym_name() {
        // Test some valid cases
        assert!(NymName::new("foo").is_ok());
        assert!(NymName::new("foo-bar").is_ok());
        assert!(NymName::new("foo-bar-123").is_ok());
        assert!(NymName::new("foo_bar").is_ok());
        assert!(NymName::new("foo_bar_123").is_ok());

        // And now test all some invalid ones
        assert!(NymName::new("Foo").is_err());
        assert!(NymName::new("foo bar").is_err());
        assert!(NymName::new("foo!bar").is_err());
        assert!(NymName::new("foo#bar").is_err());
        assert!(NymName::new("foo$bar").is_err());
        assert!(NymName::new("foo%bar").is_err());
        assert!(NymName::new("foo&bar").is_err());
        assert!(NymName::new("foo'bar").is_err());
        assert!(NymName::new("foo(bar").is_err());
        assert!(NymName::new("foo)bar").is_err());
        assert!(NymName::new("foo*bar").is_err());
        assert!(NymName::new("foo+bar").is_err());
        assert!(NymName::new("foo,bar").is_err());
        assert!(NymName::new("foo.bar").is_err());
        assert!(NymName::new("foo.bar").is_err());
        assert!(NymName::new("foo/bar").is_err());
        assert!(NymName::new("foo/bar").is_err());
        assert!(NymName::new("foo:bar").is_err());
        assert!(NymName::new("foo;bar").is_err());
        assert!(NymName::new("foo<bar").is_err());
        assert!(NymName::new("foo=bar").is_err());
        assert!(NymName::new("foo>bar").is_err());
        assert!(NymName::new("foo?bar").is_err());
        assert!(NymName::new("foo@bar").is_err());
        assert!(NymName::new("fooBar").is_err());
        assert!(NymName::new("foo[bar").is_err());
        assert!(NymName::new("foo\"bar").is_err());
        assert!(NymName::new("foo\\bar").is_err());
        assert!(NymName::new("foo]bar").is_err());
        assert!(NymName::new("foo^bar").is_err());
        assert!(NymName::new("foo`bar").is_err());
        assert!(NymName::new("foo{bar").is_err());
        assert!(NymName::new("foo|bar").is_err());
        assert!(NymName::new("foo}bar").is_err());
        assert!(NymName::new("foo~bar").is_err());
    }
}
