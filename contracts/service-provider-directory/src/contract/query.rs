use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

use crate::{
    error::Result,
    msg::{ConfigResponse, PagedServicesListResponse, ServiceInfo, ServicesListResponse},
    state,
    types::ServiceId,
};

const SERVICE_DEFAULT_RETRIEVAL_LIMIT: u32 = 100;
const SERVICE_MAX_RETRIEVAL_LIMIT: u32 = 150;

pub fn query_id(deps: Deps, service_id: ServiceId) -> Result<ServiceInfo> {
    let service = state::services().load(deps.storage, service_id)?;
    Ok(ServiceInfo {
        service_id,
        service,
    })
}

pub fn query_all_paged(
    deps: Deps,
    start_after: Option<ServiceId>,
    limit: Option<u32>,
) -> StdResult<ServicesListResponse> {
    // WIP(JON)
    //) -> StdResult<PagerServicesListResponse> {
    let limit = limit
        .unwrap_or(SERVICE_DEFAULT_RETRIEVAL_LIMIT)
        .min(SERVICE_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let services = state::services()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(ServicesListResponse::new(services))
}

pub fn query_config(deps: Deps) -> Result<ConfigResponse> {
    let config = state::load_config(deps.storage)?;
    Ok(config.into())
}
