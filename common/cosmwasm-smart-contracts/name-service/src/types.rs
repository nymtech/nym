// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use nym_contracts_common::IdentityKey;
use std::fmt::{Display, Formatter};

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
    NymAddress(String),
    // Possible extension:
    //Gateway(String)
}

impl Address {
    /// Create a new nym address.
    pub fn new(address: &str) -> Result<Self> {
        string_is_valid_nym_address(address)
            .then(|| Self::NymAddress(address.to_string()))
            .ok_or_else(|| NameServiceError::InvalidNymAddress(address.to_string()))
    }

    pub fn as_str(&self) -> &str {
        match self {
            Address::NymAddress(address) => address,
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
        write!(f, "{}", self.as_str())
    }
}

// A valid nym address is of the form client_id.client_enc@gateway_id
fn string_is_valid_nym_address(address: &str) -> bool {
    let parts: Vec<&str> = address.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let client_part = parts[0];
    let gateway_part = parts[1];

    // The client part consists of two parts separated by a dot
    let client_parts: Vec<&str> = client_part.split('.').collect();
    if client_parts.len() != 2 {
        return false;
    }

    // Check that the gateway part does not contain any dots
    if gateway_part.contains('.') {
        return false;
    }

    true
}

/// Name stored and pointing a to a nym-address
#[cw_serde]
pub struct NymName(String);

#[derive(Debug)]
pub enum NymNameError {
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
