use cosmwasm_std::{from_binary, testing::mock_env, Addr, Deps, StdError};

use crate::{
    error::ContractError,
    msg::{ConfigResponse, QueryMsg, ServiceInfo, ServicesListResponse},
    state::ServiceId,
};

pub fn assert_config(deps: Deps, admin: Addr) {
    let res = crate::contract::query(deps, mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(config, ConfigResponse { admin });
}

pub fn assert_services(deps: Deps, expected_services: &[ServiceInfo]) {
    let res = crate::contract::query(deps, mock_env(), QueryMsg::all()).unwrap();
    let services: ServicesListResponse = from_binary(&res).unwrap();
    assert_eq!(
        services,
        ServicesListResponse {
            services: expected_services.to_vec(),
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
    let services: ServicesListResponse = from_binary(&res).unwrap();
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
