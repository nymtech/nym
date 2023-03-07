use crate::{IdentityKey, IdentityKeyRef};
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
