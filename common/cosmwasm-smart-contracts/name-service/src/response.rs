use crate::{NameId, RegisteredName};
use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NamesListResponse {
    pub names: Vec<RegisteredName>,
}

impl NamesListResponse {
    pub fn new(names: Vec<RegisteredName>) -> NamesListResponse {
        NamesListResponse { names }
    }
}

impl From<&[RegisteredName]> for NamesListResponse {
    fn from(names: &[RegisteredName]) -> Self {
        NamesListResponse {
            names: names.to_vec(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PagedNamesListResponse {
    pub names: Vec<RegisteredName>,
    pub per_page: usize,
    pub start_next_after: Option<NameId>,
}

impl PagedNamesListResponse {
    pub fn new(
        names: Vec<RegisteredName>,
        per_page: usize,
        start_next_after: Option<NameId>,
    ) -> PagedNamesListResponse {
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
