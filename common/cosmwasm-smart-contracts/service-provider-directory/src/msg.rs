use crate::{NymAddress, ServiceDetails, ServiceId};
use cosmwasm_std::Coin;
use nym_contracts_common::signing::MessageSignature;
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

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ServiceId {
        service_id: ServiceId,
    },
    ByAnnouncer {
        announcer: String,
    },
    ByNymAddress {
        nym_address: NymAddress,
    },
    All {
        limit: Option<u32>,
        start_after: Option<ServiceId>,
    },
    SigningNonce {
        address: String,
    },
    Config {},

    /// Gets build information of this contract, such as the commit hash used for the build or rustc version.
    GetContractVersion {},

    /// Gets the stored contract version information that's required by the CW2 spec interface for migrations.
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
