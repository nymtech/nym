use crate::state::Config;
use crate::types::{NymAddress, Service, ServiceId, ServiceType};
use cosmwasm_std::{Addr, Coin};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
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
    ServiceId {
        service_id: ServiceId,
    },
    All {
        limit: Option<u32>,
        start_after: Option<ServiceId>,
    },
    Config {},
}

impl QueryMsg {
    pub fn all() -> QueryMsg {
        QueryMsg::All {
            limit: None,
            start_after: None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ServiceInfo {
    pub service_id: ServiceId,
    pub service: Service,
}

impl ServiceInfo {
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
impl ServicesListResponse {
    pub(crate) fn new(services: Vec<(ServiceId, Service)>) -> ServicesListResponse {
        let s = services
            .into_iter()
            .map(|(service_id, service)| ServiceInfo::new(service_id, service))
            .collect();
        ServicesListResponse { services: s }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PagedServicesListResponse {
    pub services: Vec<ServiceInfo>,
    pub per_page: usize,
    pub start_next_after: Option<ServiceId>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub admin: Addr,
}

impl From<Config> for ConfigResponse {
    fn from(config: Config) -> Self {
        ConfigResponse {
            admin: config.admin,
        }
    }
}
