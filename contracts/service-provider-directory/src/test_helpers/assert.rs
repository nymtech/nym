use cosmwasm_std::{from_binary, testing::mock_env, Addr, Coin, Deps, StdError};

use crate::{
    constants::SERVICE_DEFAULT_RETRIEVAL_LIMIT,
    error::ContractError,
    msg::{ConfigResponse, PagedServicesListResponse, QueryMsg, ServiceInfo},
    types::ServiceId,
};

pub fn assert_config(deps: Deps, admin: Addr, deposit_required: Coin) {
    let res = crate::contract::query(deps, mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(config, ConfigResponse { deposit_required });
    // WIP(JON) also assert owner
}

pub fn assert_services(deps: Deps, expected_services: &[ServiceInfo]) {
    let res = crate::contract::query(deps, mock_env(), QueryMsg::all()).unwrap();
    let services: PagedServicesListResponse = from_binary(&res).unwrap();
    let start_next_after = expected_services.iter().last().map(|s| s.service_id);
    assert_eq!(
        services,
        PagedServicesListResponse {
            services: expected_services.to_vec(),
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after,
        }
    );
}

pub fn assert_service(deps: Deps, expected_service: &ServiceInfo) {
    let res = crate::contract::query(
        deps,
        mock_env(),
        QueryMsg::ServiceId {
            service_id: expected_service.service_id,
        },
    )
    .unwrap();
    let services: ServiceInfo = from_binary(&res).unwrap();
    assert_eq!(&services, expected_service);
}

pub fn assert_empty(deps: Deps) {
    let res = crate::contract::query(deps, mock_env(), QueryMsg::all()).unwrap();
    let services: PagedServicesListResponse = from_binary(&res).unwrap();
    assert!(services.services.is_empty());
}

pub fn assert_not_found(deps: Deps, expected_id: ServiceId) {
    let res = crate::contract::query(
        deps,
        mock_env(),
        QueryMsg::ServiceId {
            service_id: expected_id,
        },
    )
    .unwrap_err();
    assert!(matches!(res, ContractError::Std(StdError::NotFound { .. })));
}
