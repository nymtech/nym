use crate::state::{Config, NymAddress, Service, ServiceId, ServiceType};
use cosmwasm_std::{Addr, Coin};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub updater_role: Addr,
    pub admin: Addr,
    pub deposit_required: Coin,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Announce {
        nym_address: NymAddress,
        service_type: ServiceType,
        owner: Addr,
    },
    Delete {
        service_id: ServiceId,
    },
}

impl ExecuteMsg {
    pub fn delete(service_id: ServiceId) -> Self {
        ExecuteMsg::Delete { service_id }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryId { service_id: ServiceId },
    QueryAll {},
    QueryConfig {},
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ServiceInfo {
    pub service_id: ServiceId,
    pub service: Service,
}

impl ServiceInfo {
    #[cfg(test)]
    pub fn new(service_id: ServiceId, service: Service) -> Self {
        Self {
            service_id,
            service,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ServicesListResponse {
    pub services: Vec<ServiceInfo>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub updater_role: Addr,
    pub admin: Addr,
}

impl From<Config> for ConfigResponse {
    fn from(config: Config) -> Self {
        ConfigResponse {
            updater_role: config.updater_role,
            admin: config.admin,
        }
    }
}
