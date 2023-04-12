use crate::{msg::ExecuteMsg, Service, ServiceId, ServiceInfo};
use cosmwasm_std::Coin;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ServiceInfoResponse {
    pub service_id: ServiceId,
    pub service: Option<Service>,
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
    pub per_page: usize,
    pub start_next_after: Option<ServiceId>,
}

impl PagedServicesListResponse {
    pub fn new(
        services: Vec<(ServiceId, Service)>,
        per_page: usize,
        start_next_after: Option<ServiceId>,
    ) -> PagedServicesListResponse {
        let services = services
            .into_iter()
            .map(|(service_id, service)| ServiceInfo::new(service_id, service))
            .collect();
        PagedServicesListResponse {
            services,
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

impl From<Service> for ExecuteMsg {
    fn from(service: Service) -> Self {
        ExecuteMsg::Announce {
            nym_address: service.nym_address,
            service_type: service.service_type,
        }
    }
}
