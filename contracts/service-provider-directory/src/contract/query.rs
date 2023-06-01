use cosmwasm_std::Deps;
use nym_contracts_common::{signing::Nonce, ContractBuildInformation};
use nym_service_provider_directory_common::{
    response::{ConfigResponse, PagedServicesListResponse, ServicesListResponse},
    NymAddress, Service, ServiceId,
};

use crate::{
    state::{self, PagedLoad},
    Result,
};

pub fn query_id(deps: Deps, service_id: ServiceId) -> Result<Service> {
    state::load_id(deps.storage, service_id)
}

pub fn query_announcer(deps: Deps, announcer: String) -> Result<ServicesListResponse> {
    let announcer = deps.api.addr_validate(&announcer)?;
    let services = state::load_announcer(deps.storage, announcer)?;
    Ok(ServicesListResponse::new(services))
}

pub fn query_nym_address(deps: Deps, nym_address: NymAddress) -> Result<ServicesListResponse> {
    let services = state::load_nym_address(deps.storage, nym_address)?;
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
    } = state::load_all_paged(deps.storage, limit, start_after)?;
    Ok(PagedServicesListResponse::new(
        services,
        limit,
        start_next_after,
    ))
}

pub fn query_current_signing_nonce(deps: Deps<'_>, address: String) -> Result<Nonce> {
    let address = deps.api.addr_validate(&address)?;
    state::get_signing_nonce(deps.storage, address)
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
