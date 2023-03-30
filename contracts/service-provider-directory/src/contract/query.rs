use cosmwasm_std::{Addr, Deps};
use nym_service_provider_directory_common::{
    msg::{ConfigResponse, PagedServicesListResponse, ServiceInfo, ServicesListResponse},
    NymAddress, ServiceId,
};

use crate::{error::Result, state};

pub fn query_id(deps: Deps, service_id: ServiceId) -> Result<ServiceInfo> {
    let service = state::services::load_id(deps.storage, service_id)?;
    Ok(ServiceInfo {
        service_id,
        service,
    })
}

pub fn query_owner(deps: Deps, owner: Addr) -> Result<ServicesListResponse> {
    let services = state::services::load_owner(deps.storage, owner)?;
    Ok(ServicesListResponse::new(services))
}

pub fn query_nym_address(deps: Deps, nym_address: NymAddress) -> Result<ServicesListResponse> {
    let services = state::services::load_nym_address(deps.storage, nym_address)?;
    Ok(ServicesListResponse::new(services))
}

pub fn query_all_paged(
    deps: Deps,
    start_after: Option<ServiceId>,
    limit: Option<u32>,
) -> Result<PagedServicesListResponse> {
    let (services, start_next_after, limit) =
        state::services::load_all_paged(deps.storage, start_after, limit)?;
    Ok(PagedServicesListResponse::new(
        services,
        start_next_after,
        limit,
    ))
}

pub fn query_config(deps: Deps) -> Result<ConfigResponse> {
    let config = state::load_config(deps.storage)?;
    Ok(config.into())
}
