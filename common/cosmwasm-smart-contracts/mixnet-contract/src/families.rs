use crate::{IdentityKey, IdentityKeyRef};
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/NodeFamily.ts")
)]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, JsonSchema)]
pub struct Family {
    head: FamilyHead,
    proxy: Option<String>,
    label: String,
}

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
    pub fn new(identity: IdentityKeyRef<'_>) -> Self {
        FamilyHead(identity.to_string())
    }

    pub fn identity(&self) -> IdentityKeyRef<'_> {
        &self.0
    }
}

impl Family {
    pub fn new(head: FamilyHead, proxy: Option<Addr>, label: String) -> Self {
        Family {
            head,
            proxy: proxy.map(|p| p.to_string()),
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
