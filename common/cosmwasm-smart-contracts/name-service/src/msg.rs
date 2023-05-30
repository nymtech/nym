use crate::{Address, NameId, NymName};
use cosmwasm_std::Coin;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub deposit_required: Coin,
}

impl InstantiateMsg {
    pub fn new(deposit_required: Coin) -> Self {
        Self { deposit_required }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Announcing a name pointing to a nym-address
    Register { name: NymName, address: Address },
    /// Delete a name entry by id
    DeleteId { name_id: NameId },
    /// Delete a name entry by name
    DeleteName { name: NymName },
    /// Change the deposit required for announcing a name
    UpdateDepositRequired { deposit_required: Coin },
}

impl ExecuteMsg {
    pub fn delete_id(name_id: NameId) -> Self {
        ExecuteMsg::DeleteId { name_id }
    }

    pub fn default_memo(&self) -> String {
        match self {
            ExecuteMsg::Register { name, address } => {
                format!("registering {address} as name: {name}")
            }
            ExecuteMsg::DeleteId { name_id } => {
                format!("deleting name with id {name_id}")
            }
            ExecuteMsg::DeleteName { name } => {
                format!("deleting name: {name}")
            }
            ExecuteMsg::UpdateDepositRequired { deposit_required } => {
                format!("updating the deposit required to {deposit_required}")
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query the name by it's assigned id
    NameId {
        name_id: NameId,
    },
    // Query the names by the registrator
    ByOwner {
        owner: String,
    },
    ByName {
        name: NymName,
    },
    ByAddress {
        address: Address,
    },
    All {
        limit: Option<u32>,
        start_after: Option<NameId>,
    },
    Config {},
    GetContractVersion {},
    #[serde(rename = "get_cw2_contract_version")]
    GetCW2ContractVersion {},
}

impl QueryMsg {
    pub fn all() -> QueryMsg {
        QueryMsg::All {
            limit: None,
            start_after: None,
        }
    }
}
