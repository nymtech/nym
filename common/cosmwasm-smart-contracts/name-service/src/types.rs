use std::fmt::{Display, Formatter};

use cosmwasm_std::{Addr, Coin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The directory of services are indexed by [`ServiceId`].
pub type NameId = u32;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct RegisteredName {
    /// The name pointing to the nym address
    pub name: NymName,
    /// The address of the service.
    pub address: Address,
    /// Service owner.
    pub owner: Addr,
    /// Block height at which the service was added.
    pub block_height: u64,
    /// The deposit used to announce the service.
    pub deposit: Coin,
}

/// String representation of a nym address, which is of the form
/// client_id.client_enc@gateway_id.
/// NOTE: entirely unvalidated.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Address {
    NymAddress(String),
    // Possible extension:
    //Gateway(String)
}

impl Address {
    /// Create a new nym address.
    pub fn new(address: &str) -> Self {
        Self::NymAddress(address.to_string())
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

/// Name stored and pointing a to a nym-address
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NymName(String);

#[derive(Debug)]
pub enum NymNameError {
    InvalidName,
}

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

/// [`RegisterdName`] together with the assigned [`NameId`].
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NameEntry {
    pub name_id: NameId,
    pub name: RegisteredName,
}

impl NameEntry {
    pub fn new(name_id: NameId, name: RegisteredName) -> Self {
        Self { name_id, name }
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
