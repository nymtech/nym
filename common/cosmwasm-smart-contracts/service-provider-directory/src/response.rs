use crate::{Service, ServiceId};
use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ServiceInfoResponse {
    pub service_id: ServiceId,
    pub service: Option<Service>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ServicesListResponse {
    pub services: Vec<Service>,
}

impl ServicesListResponse {
    pub fn new(services: Vec<Service>) -> ServicesListResponse {
        ServicesListResponse { services }
    }
}

impl From<&[Service]> for ServicesListResponse {
    fn from(services: &[Service]) -> Self {
        Self {
            services: services.to_vec(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PagedServicesListResponse {
    pub services: Vec<Service>,
    pub per_page: usize,
    pub start_next_after: Option<ServiceId>,
}

impl PagedServicesListResponse {
    pub fn new(
        services: Vec<Service>,
        per_page: usize,
        start_next_after: Option<ServiceId>,
    ) -> PagedServicesListResponse {
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
