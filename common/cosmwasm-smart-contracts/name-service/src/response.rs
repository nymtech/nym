use crate::{msg::ExecuteMsg, NameEntry, NameId, RegisteredName};
use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Like [`NameEntry`] but since it's a response type the name is an option depending on if
/// the name exists or not.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct NameEntryResponse {
    pub name_id: NameId,
    pub name: Option<RegisteredName>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NamesListResponse {
    pub names: Vec<NameEntry>,
}

impl NamesListResponse {
    pub fn new(names: Vec<(NameId, RegisteredName)>) -> NamesListResponse {
        NamesListResponse {
            names: names
                .into_iter()
                .map(|(name_id, name)| NameEntry::new(name_id, name))
                .collect(),
        }
    }
}

impl From<&[NameEntry]> for NamesListResponse {
    fn from(names: &[NameEntry]) -> Self {
        NamesListResponse {
            names: names.to_vec(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PagedNamesListResponse {
    pub names: Vec<NameEntry>,
    pub per_page: usize,
    pub start_next_after: Option<NameId>,
}

impl PagedNamesListResponse {
    pub fn new(
        names: Vec<(NameId, RegisteredName)>,
        per_page: usize,
        start_next_after: Option<NameId>,
    ) -> PagedNamesListResponse {
        let names = names
            .into_iter()
            .map(|(name_id, name)| NameEntry::new(name_id, name))
            .collect();
        PagedNamesListResponse {
            names,
            per_page,
            start_next_after,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub deposit_required: Coin,
}

impl From<RegisteredName> for ExecuteMsg {
    fn from(name: RegisteredName) -> Self {
        ExecuteMsg::Register {
            name: name.name,
            address: name.address,
        }
    }
}
