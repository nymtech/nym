use cosmwasm_std::Deps;
use cw_storage_plus::Bound;

use crate::{
    error::Result,
    msg::{ConfigResponse, ServiceInfo, ServicesListResponse},
    state::{self, ServiceId},
};

const SERVICE_DEFAULT_RETRIEVAL_LIMIT: u32 = 100;
const SERVICE_MAX_RETRIEVAL_LIMIT: u32 = 150;

pub fn query_id(deps: Deps, service_id: ServiceId) -> Result<ServiceInfo> {
    let service = state::load_service(deps.storage, service_id)?;
    Ok(ServiceInfo {
        service_id,
        service,
    })
}

pub fn query_all_paged(
    deps: Deps,
    limit: Option<u32>,
    start_after: Option<ServiceId>,
) -> Result<ServicesListResponse> {
    let limit = limit
        .unwrap_or(SERVICE_DEFAULT_RETRIEVAL_LIMIT)
        .min(SERVICE_MAX_RETRIEVAL_LIMIT) as usize;

    let start: Option<Bound<ServiceId>> = start_after.map(Bound::exclusive);

    let services = state::all_services(deps.storage)?;
    Ok(ServicesListResponse { services })
}

pub fn query_config(deps: Deps) -> Result<ConfigResponse> {
    let config = state::load_config(deps.storage)?;
    Ok(config.into())
}
