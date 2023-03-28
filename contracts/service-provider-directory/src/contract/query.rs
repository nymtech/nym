use cosmwasm_std::{Deps, StdResult};

use crate::{
    msg::{ConfigResponse, ServiceInfo, ServicesListResponse},
    state::{self, ServiceId},
};

pub fn query_id(deps: Deps, service_id: ServiceId) -> StdResult<ServiceInfo> {
    let service = state::load_service(deps.storage, service_id)?;
    Ok(ServiceInfo {
        service_id,
        service,
    })
}

pub fn query_all(deps: Deps) -> StdResult<ServicesListResponse> {
    let services = state::load_all_services(deps.storage)?;
    Ok(ServicesListResponse { services })
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = state::load_config(deps.storage)?;
    Ok(config.into())
}
