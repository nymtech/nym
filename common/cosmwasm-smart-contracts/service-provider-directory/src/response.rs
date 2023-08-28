// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{Service, ServiceId};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

#[cw_serde]
pub struct ServiceInfoResponse {
    pub service_id: ServiceId,
    pub service: Option<Service>,
}

#[cw_serde]
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

#[cw_serde]
pub struct PagedServicesListResponse {
    pub services: Vec<Service>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
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

#[cw_serde]
pub struct ConfigResponse {
    pub deposit_required: Coin,
}
