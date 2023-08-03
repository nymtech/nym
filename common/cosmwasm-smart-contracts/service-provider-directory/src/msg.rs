// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{NymAddress, ServiceDetails, ServiceId};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use nym_contracts_common::signing::MessageSignature;

#[cfg(feature = "schema")]
use crate::{
    response::{ConfigResponse, PagedServicesListResponse, ServicesListResponse},
    types::Service,
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
    Announce {
        service: ServiceDetails,
        owner_signature: MessageSignature,
    },
    DeleteId {
        service_id: ServiceId,
    },
    DeleteNymAddress {
        nym_address: NymAddress,
    },
    UpdateDepositRequired {
        deposit_required: Coin,
    },
}

impl ExecuteMsg {
    pub fn delete_id(service_id: ServiceId) -> Self {
        ExecuteMsg::DeleteId { service_id }
    }

    pub fn default_memo(&self) -> String {
        match self {
            ExecuteMsg::Announce {
                service,
                owner_signature: _,
            } => format!(
                "announcing {} as type {}",
                service.nym_address, service.service_type
            ),
            ExecuteMsg::DeleteId { service_id } => {
                format!("deleting service with service id {service_id}")
            }
            ExecuteMsg::DeleteNymAddress { nym_address } => {
                format!("deleting service with nym address {nym_address}")
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
    #[cfg_attr(feature = "schema", returns(Service))]
    ServiceId { service_id: ServiceId },

    #[cfg_attr(feature = "schema", returns(ServicesListResponse))]
    ByAnnouncer { announcer: String },

    #[cfg_attr(feature = "schema", returns(ServicesListResponse))]
    ByNymAddress { nym_address: NymAddress },

    #[cfg_attr(feature = "schema", returns(PagedServicesListResponse))]
    All {
        limit: Option<u32>,
        start_after: Option<ServiceId>,
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
