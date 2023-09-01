// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{Address, NameDetails, NameId, NymName};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use nym_contracts_common::signing::MessageSignature;

#[cfg(feature = "schema")]
use crate::{
    response::{ConfigResponse, NamesListResponse, PagedNamesListResponse},
    types::RegisteredName,
};
#[cfg(feature = "schema")]
use cosmwasm_schema::QueryResponses;
#[cfg(feature = "schema")]
use nym_contracts_common::{signing::Nonce, ContractBuildInformation};

#[cw_serde]
pub struct InstantiateMsg {
    pub deposit_required: Coin,
}

impl InstantiateMsg {
    pub fn new(deposit_required: Coin) -> Self {
        Self { deposit_required }
    }
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Announcing a name pointing to a nym-address
    Register {
        name: NameDetails,
        owner_signature: MessageSignature,
    },

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
            ExecuteMsg::Register {
                name,
                owner_signature: _,
            } => {
                format!("registering {} as name: {}", name.address, name.name)
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

#[cw_serde]
#[cfg_attr(feature = "schema", derive(QueryResponses))]
pub enum QueryMsg {
    /// Query the name by it's assigned id
    #[cfg_attr(feature = "schema", returns(RegisteredName))]
    NameId { name_id: NameId },

    /// Query the names by the registrator
    #[cfg_attr(feature = "schema", returns(NamesListResponse))]
    ByOwner { owner: String },

    #[cfg_attr(feature = "schema", returns(RegisteredName))]
    ByName { name: NymName },

    #[cfg_attr(feature = "schema", returns(NamesListResponse))]
    ByAddress { address: Address },

    #[cfg_attr(feature = "schema", returns(PagedNamesListResponse))]
    All {
        limit: Option<u32>,
        start_after: Option<NameId>,
    },

    #[cfg_attr(feature = "schema", returns(Nonce))]
    SigningNonce { address: String },

    #[cfg_attr(feature = "schema", returns(ConfigResponse))]
    Config {},

    /// Gets build information of this contract, such as the commit hash used for the build or rustc version.
    #[cfg_attr(feature = "schema", returns(ContractBuildInformation))]
    GetContractVersion {},

    /// Gets the stored contract version information that's required by the CW2 spec interface for migrations.
    #[serde(rename = "get_cw2_contract_version")]
    #[cfg_attr(feature = "schema", returns(cw2::ContractVersion))]
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
