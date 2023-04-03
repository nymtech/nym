use crate::{NymAddress, Service, ServiceId, ServiceType};
use cosmwasm_std::{Addr, Coin};
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
pub enum ExecuteMsg {
    Announce {
        nym_address: NymAddress,
        service_type: ServiceType,
    },
    Delete {
        service_id: ServiceId,
    },
    UpdateDepositRequired {
        deposit_required: Coin,
    },
}

impl ExecuteMsg {
    pub fn delete(service_id: ServiceId) -> Self {
        ExecuteMsg::Delete { service_id }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ServiceId {
        service_id: ServiceId,
    },
    Owner {
        owner: Addr,
    },
    NymAddress {
        nym_address: NymAddress,
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
    pub fn new(services: Vec<(ServiceId, Service)>) -> ServicesListResponse {
        ServicesListResponse {
            services: services
                .into_iter()
                .map(|(service_id, service)| ServiceInfo::new(service_id, service))
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PagedServicesListResponse {
    pub services: Vec<ServiceInfo>,
    pub start_next_after: Option<ServiceId>,
    pub per_page: usize,
}

impl PagedServicesListResponse {
    pub fn new(
        services: Vec<(ServiceId, Service)>,
        start_next_after: Option<ServiceId>,
        per_page: usize,
    ) -> PagedServicesListResponse {
        let services = services
            .into_iter()
            .map(|(service_id, service)| ServiceInfo::new(service_id, service))
            .collect();
        PagedServicesListResponse {
            services,
            start_next_after,
            per_page,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub deposit_required: Coin,
}

impl From<Service> for ExecuteMsg {
    fn from(service: Service) -> Self {
        ExecuteMsg::Announce {
            nym_address: service.nym_address,
            service_type: service.service_type,
        }
    }
}
