// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{retrieval_limits, NYM_POOL_STORAGE};
use cosmwasm_std::{Coin, Deps, Env, Order, StdResult};
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_pool_contract_common::{
    AvailableTokensResponse, GrantInformation, GrantResponse, GranterDetails, GranterResponse,
    GrantersPagedResponse, GrantsPagedResponse, LockedTokens, LockedTokensPagedResponse,
    LockedTokensResponse, NymPoolContractError, TotalLockedTokensResponse,
};

pub fn query_admin(deps: Deps) -> Result<AdminResponse, NymPoolContractError> {
    NYM_POOL_STORAGE
        .contract_admin
        .query_admin(deps)
        .map_err(Into::into)
}

pub fn query_available_tokens(
    deps: Deps,
    env: Env,
) -> Result<AvailableTokensResponse, NymPoolContractError> {
    Ok(AvailableTokensResponse {
        available: NYM_POOL_STORAGE.available_tokens(deps, &env)?,
    })
}

pub fn query_total_locked_tokens(
    deps: Deps,
) -> Result<TotalLockedTokensResponse, NymPoolContractError> {
    let denom = NYM_POOL_STORAGE.pool_denomination.load(deps.storage)?;
    let amount = NYM_POOL_STORAGE.locked.total_locked.load(deps.storage)?;
    Ok(TotalLockedTokensResponse {
        locked: Coin::new(amount, denom),
    })
}

pub fn query_locked_tokens(
    deps: Deps,
    grantee: String,
) -> Result<LockedTokensResponse, NymPoolContractError> {
    let grantee = deps.api.addr_validate(&grantee)?;
    let denom = NYM_POOL_STORAGE.pool_denomination.load(deps.storage)?;
    let amount = NYM_POOL_STORAGE
        .locked
        .maybe_grantee_locked(deps.storage, grantee.clone())?;

    Ok(LockedTokensResponse {
        locked: amount.map(|amount| Coin::new(amount, denom)),
        grantee,
    })
}

pub fn query_locked_tokens_paged(
    deps: Deps,
    limit: Option<u32>,
    start_after: Option<String>,
) -> Result<LockedTokensPagedResponse, NymPoolContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::LOCKED_TOKENS_DEFAULT_LIMIT)
        .min(retrieval_limits::LOCKED_TOKENS_MAX_LIMIT) as usize;
    let grantee = start_after
        .map(|grantee| deps.api.addr_validate(&grantee))
        .transpose()?;
    let denom = NYM_POOL_STORAGE.pool_denomination.load(deps.storage)?;

    let start = grantee.map(Bound::exclusive);

    let locked = NYM_POOL_STORAGE
        .locked
        .grantees
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(grantee, amount)| LockedTokens {
                grantee,
                locked: Coin::new(amount, &denom),
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = locked.last().map(|locked| locked.grantee.to_string());

    Ok(LockedTokensPagedResponse {
        locked,
        start_next_after,
    })
}

pub fn query_grant(
    deps: Deps,
    env: Env,
    grantee: String,
) -> Result<GrantResponse, NymPoolContractError> {
    let grantee = deps.api.addr_validate(&grantee)?;
    let grant = NYM_POOL_STORAGE.try_load_grant(deps, &grantee)?;

    Ok(GrantResponse {
        grant: grant.map(|grant| GrantInformation {
            expired: grant.allowance.expired(&env),
            grant,
        }),
        grantee,
    })
}

pub fn query_granter(deps: Deps, granter: String) -> Result<GranterResponse, NymPoolContractError> {
    let granter = deps.api.addr_validate(&granter)?;

    Ok(GranterResponse {
        information: NYM_POOL_STORAGE.try_load_granter(deps, &granter)?,
        granter,
    })
}

pub fn query_grants_paged(
    deps: Deps,
    env: Env,
    limit: Option<u32>,
    start_after: Option<String>,
) -> Result<GrantsPagedResponse, NymPoolContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::GRANTERS_DEFAULT_LIMIT)
        .min(retrieval_limits::GRANTERS_MAX_LIMIT) as usize;
    let grantee = start_after
        .map(|grantee| deps.api.addr_validate(&grantee))
        .transpose()?;

    let start = grantee.map(Bound::exclusive);

    let grants = NYM_POOL_STORAGE
        .grants
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(_, grant)| GrantInformation {
                expired: grant.allowance.expired(&env),
                grant,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = grants.last().map(|info| info.grant.grantee.to_string());

    Ok(GrantsPagedResponse {
        grants,
        start_next_after,
    })
}

pub fn query_granters_paged(
    deps: Deps,
    limit: Option<u32>,
    start_after: Option<String>,
) -> Result<GrantersPagedResponse, NymPoolContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::GRANTERS_DEFAULT_LIMIT)
        .min(retrieval_limits::GRANTERS_MAX_LIMIT) as usize;
    let granter = start_after
        .map(|granter| deps.api.addr_validate(&granter))
        .transpose()?;
    let start = granter.map(Bound::exclusive);

    let granters = NYM_POOL_STORAGE
        .granters
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(granter, info)| GranterDetails::from((granter, info))))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = granters.last().map(|details| details.granter.to_string());

    Ok(GrantersPagedResponse {
        granters,
        start_next_after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod admin_query {
        use super::*;
        use crate::testing::TestSetup;
        use nym_pool_contract_common::ExecuteMsg;

        #[test]
        fn returns_current_admin() -> anyhow::Result<()> {
            let mut test = TestSetup::init();

            let initial_admin = test.admin_unchecked();

            // initial
            let res = query_admin(test.deps())?;
            assert_eq!(res.admin, Some(initial_admin.to_string()));

            let new_admin = test.generate_account();

            // sanity check
            assert_ne!(initial_admin, new_admin);

            // after update
            test.execute_msg(
                initial_admin.clone(),
                &ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                },
            )?;

            let updated_admin = query_admin(test.deps())?;
            assert_eq!(updated_admin.admin, Some(new_admin.to_string()));

            Ok(())
        }
    }

    #[cfg(test)]
    mod available_tokens_query {
        use super::*;

        #[test]
        fn todo() {
            todo!()
        }
    }

    #[cfg(test)]
    mod total_locked_tokens_query {
        use super::*;

        #[test]
        fn todo() {
            todo!()
        }
    }

    #[cfg(test)]
    mod locked_tokens_query {
        use super::*;

        #[test]
        fn todo() {
            todo!()
        }
    }

    #[cfg(test)]
    mod locked_tokens_paged_query {
        use super::*;

        #[test]
        fn todo() {
            todo!()
        }
    }

    #[cfg(test)]
    mod grant_query {
        use super::*;

        #[test]
        fn todo() {
            todo!()
        }
    }

    #[cfg(test)]
    mod granter_query {
        use super::*;

        #[test]
        fn todo() {
            todo!()
        }
    }

    #[cfg(test)]
    mod granters_paged_query {
        use super::*;

        #[test]
        fn todo() {
            todo!()
        }
    }

    #[cfg(test)]
    mod grants_paged_query {
        use super::*;

        #[test]
        fn todo() {
            todo!()
        }
    }
}
