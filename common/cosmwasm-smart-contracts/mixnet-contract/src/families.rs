use crate::{IdentityKey, IdentityKeyRef};
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
    members: HashSet<String>,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/NodeFamilyHead.ts")
)]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, JsonSchema)]
pub struct FamilyHead(IdentityKey);

impl FamilyHead {
    pub fn new(identity: IdentityKeyRef<'_>) -> Self {
        FamilyHead(identity.to_string())
    }

    pub fn identity(&self) -> IdentityKeyRef<'_> {
        &self.0
    }
}

impl Family {
    pub fn new(head: FamilyHead, proxy: Option<Addr>, label: &str) -> Self {
        Family {
            head,
            proxy: proxy.map(|p| p.to_string()),
            label: label.to_string(),
            members: HashSet::new(),
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

    #[allow(dead_code)]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[allow(dead_code)]
    pub fn members(&self) -> &HashSet<String> {
        &self.members
    }

    pub fn members_mut(&mut self) -> &mut HashSet<String> {
        &mut self.members
    }

    pub fn storage_key(&self) -> String {
        family_storage_key(self.head_identity(), self.proxy.as_ref())
    }

    #[allow(dead_code)]
    pub fn is_member(&self, member: IdentityKeyRef<'_>) -> bool {
        self.members().contains(member)
    }
}

pub fn family_storage_key(head: &str, proxy: Option<&String>) -> String {
    if let Some(proxy) = proxy {
        let key_bytes = head
            .as_bytes()
            .iter()
            .zip(proxy.as_bytes())
            .map(|(x, y)| x ^ y)
            .collect::<Vec<_>>();
        bs58::encode(key_bytes).into_string()
    } else {
        head.to_string()
    }
}
