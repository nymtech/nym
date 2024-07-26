// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{IdentityKey, IdentityKeyRef};
use cosmwasm_schema::cw_serde;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// A group of mixnodes associated with particular staking entity.
/// When defined all nodes belonging to the same family will be prioritised to be put onto the same layer.
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/NodeFamily.ts")
)]
#[cw_serde]
pub struct Family {
    /// Owner of this family.
    head: FamilyHead,

    /// Optional proxy (i.e. vesting contract address) used when creating the family.
    proxy: Option<String>,

    /// Human readable label for this family.
    label: String,
}

/// Head of particular family as identified by its identity key (i.e. public component of its ed25519 keypair stringified into base58).
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/NodeFamilyHead.ts")
)]
#[derive(Debug, Clone, Eq, PartialEq, JsonSchema)]
pub struct FamilyHead(IdentityKey);

impl Serialize for FamilyHead {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FamilyHead {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = IdentityKey::deserialize(deserializer)?;
        Ok(FamilyHead(inner))
    }
}

impl FromStr for FamilyHead {
    type Err = <IdentityKey as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // theoretically we should be verifying whether it's a valid base58 value
        // (or even better, whether it's a valid ed25519 public key), but definition of
        // `FamilyHead` might change later
        Ok(FamilyHead(IdentityKey::from_str(s)?))
    }
}

impl Display for FamilyHead {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FamilyHead {
    pub fn new<S: Into<String>>(identity: S) -> Self {
        FamilyHead(identity.into())
    }

    pub fn identity(&self) -> IdentityKeyRef<'_> {
        &self.0
    }
}

impl Family {
    pub fn new(head: FamilyHead, label: String) -> Self {
        Family {
            head,
            proxy: None,
            label,
        }
    }

    #[allow(dead_code)]
    pub fn head(&self) -> &FamilyHead {
        &self.head
    }

    pub fn head_identity(&self) -> IdentityKeyRef<'_> {
        self.head.identity()
    }

    #[allow(dead_code)]
    pub fn proxy(&self) -> Option<&String> {
        self.proxy.as_ref()
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Response containing paged list of all families registered in the contract.
#[cw_serde]
pub struct PagedFamiliesResponse {
    /// The families registered in the contract.
    pub families: Vec<Family>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<String>,
}

/// Response containing paged list of all family members (of ALL families) registered in the contract.
#[cw_serde]
pub struct PagedMembersResponse {
    /// The members alongside their family heads.
    pub members: Vec<(IdentityKey, FamilyHead)>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<String>,
}

/// Response containing family information.
#[cw_serde]
pub struct FamilyByHeadResponse {
    /// The family head used for the query.
    pub head: FamilyHead,

    /// If applicable, the family associated with the provided head.
    pub family: Option<Family>,
}

/// Response containing family information.
#[cw_serde]
pub struct FamilyByLabelResponse {
    /// The family label used for the query.
    pub label: String,

    /// If applicable, the family associated with the provided label.
    pub family: Option<Family>,
}

/// Response containing family members information.
#[cw_serde]
pub struct FamilyMembersByHeadResponse {
    /// The family head used for the query.
    pub head: FamilyHead,

    /// All members belonging to the specified family.
    pub members: Vec<IdentityKey>,
}

/// Response containing family members information.
#[cw_serde]
pub struct FamilyMembersByLabelResponse {
    /// The family label used for the query.
    pub label: String,

    /// All members belonging to the specified family.
    pub members: Vec<IdentityKey>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_head_serde() {
        let dummy = FamilyHead::new("foomp");

        let ser_str = serde_json_wasm::to_string(&dummy).unwrap();
        let de_str: FamilyHead = serde_json_wasm::from_str(&ser_str).unwrap();
        assert_eq!(dummy, de_str);

        let ser_bytes = serde_json_wasm::to_vec(&dummy).unwrap();
        let de_bytes: FamilyHead = serde_json_wasm::from_slice(&ser_bytes).unwrap();
        assert_eq!(dummy, de_bytes);
    }
}
