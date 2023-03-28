use super::*;
use crate::{msg::ServiceInfo, state::ServiceId};

pub fn query_id(deps: Deps, _env: Env, service_id: ServiceId) -> StdResult<ServiceInfo> {
    let service = SERVICES.load(deps.storage, service_id)?;
    Ok(ServiceInfo {
        service_id,
        service,
    })
}

pub fn query_all(deps: Deps) -> StdResult<ServicesListResponse> {
    let services = SERVICES
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            item.map(|(service_id, service)| ServiceInfo {
                service_id,
                service,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;
    Ok(ServicesListResponse { services })
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.into())
}
