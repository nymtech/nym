use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_storage_plus::Bound;
use nym_service_provider_directory_common::{NymAddress, ServiceId};

use crate::{
    constants::{
        MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS, MAX_NUMBER_OF_PROVIDERS_PER_OWNER,
        SERVICE_DEFAULT_RETRIEVAL_LIMIT, SERVICE_MAX_RETRIEVAL_LIMIT,
    },
    error::Result,
    msg::{ConfigResponse, PagedServicesListResponse, ServiceInfo, ServicesListResponse},
    state,
};

pub fn query_id(deps: Deps, service_id: ServiceId) -> Result<ServiceInfo> {
    let service = state::services().load(deps.storage, service_id)?;
    Ok(ServiceInfo {
        service_id,
        service,
    })
}

pub fn query_owner(deps: Deps, owner: Addr) -> Result<ServicesListResponse> {
    let services = state::services()
        .idx
        .owner
        .prefix(owner)
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_PROVIDERS_PER_OWNER as usize)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(ServicesListResponse::new(services))
}

pub fn query_nym_address(deps: Deps, nym_address: NymAddress) -> Result<ServicesListResponse> {
    let services = state::services()
        .idx
        .nym_address
        .prefix(nym_address.to_string())
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS as usize)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(ServicesListResponse::new(services))
}

pub fn query_all_paged(
    deps: Deps,
    start_after: Option<ServiceId>,
    limit: Option<u32>,
) -> Result<PagedServicesListResponse> {
    let limit = limit
        .unwrap_or(SERVICE_DEFAULT_RETRIEVAL_LIMIT)
        .min(SERVICE_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let services = state::services()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = services.last().map(|service| service.0);
    Ok(PagedServicesListResponse::new(
        services,
        limit,
        start_next_after,
    ))
}

pub fn query_config(deps: Deps) -> Result<ConfigResponse> {
    let config = state::load_config(deps.storage)?;
    Ok(config.into())
}
