use cosmwasm_std::Deps;
use nym_contracts_common::ContractBuildInformation;
use nym_name_service_common::{
    response::{ConfigResponse, NamesListResponse, PagedNamesListResponse},
    NameEntry, NameId, NymAddress, NymName,
};

use crate::{
    error::Result,
    state::{self, names::PagedLoad},
};

pub fn query_id(deps: Deps, name_id: NameId) -> Result<NameEntry> {
    let name = state::names::load_id(deps.storage, name_id)?;
    Ok(NameEntry { name_id, name })
}

pub fn query_owner(deps: Deps, owner: String) -> Result<NamesListResponse> {
    let owner = deps.api.addr_validate(&owner)?;
    let names = state::names::load_owner(deps.storage, owner)?;
    Ok(NamesListResponse::new(names))
}

pub fn query_nym_address(deps: Deps, nym_address: NymAddress) -> Result<NamesListResponse> {
    let names = state::names::load_nym_address(deps.storage, &nym_address)?;
    Ok(NamesListResponse::new(names))
}

pub fn query_name(deps: Deps, name: NymName) -> Result<NameEntry> {
    state::names::load_name_entry(deps.storage, &name)
        .map(|(name_id, name)| NameEntry::new(name_id, name))
}

pub fn query_all_paged(
    deps: Deps,
    limit: Option<u32>,
    start_after: Option<NameId>,
) -> Result<PagedNamesListResponse> {
    let PagedLoad {
        names,
        limit,
        start_next_after,
    } = state::names::load_all_paged(deps.storage, limit, start_after)?;
    Ok(PagedNamesListResponse::new(names, limit, start_next_after))
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
