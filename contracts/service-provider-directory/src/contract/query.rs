use cosmwasm_std::{Addr, Deps};
use nym_contracts_common::ContractBuildInformation;
use nym_service_provider_directory_common::{
    msg::{ConfigResponse, PagedServicesListResponse, ServiceInfo, ServicesListResponse},
    NymAddress, ServiceId,
};

use crate::{
    error::Result,
    state::{self, services::PagedLoad},
};

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
    limit: Option<u32>,
    start_after: Option<ServiceId>,
) -> Result<PagedServicesListResponse> {
    let PagedLoad {
        services,
        limit,
        start_next_after,
    } = state::services::load_all_paged(deps.storage, limit, start_after)?;
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

pub fn query_contract_version() -> ContractBuildInformation {
    // as per docs
    // env! macro will expand to the value of the named environment variable at
    // compile time, yielding an expression of type `&'static str`
    ContractBuildInformation {
        build_timestamp: env!("VERGEN_BUILD_TIMESTAMP").to_string(),
        build_version: env!("VERGEN_BUILD_SEMVER").to_string(),
        commit_sha: option_env!("VERGEN_GIT_SHA").unwrap_or("NONE").to_string(),
        commit_timestamp: option_env!("VERGEN_GIT_COMMIT_TIMESTAMP")
            .unwrap_or("NONE")
            .to_string(),
        commit_branch: option_env!("VERGEN_GIT_BRANCH")
            .unwrap_or("NONE")
            .to_string(),
        rustc_version: env!("VERGEN_RUSTC_SEMVER").to_string(),
    }
}
