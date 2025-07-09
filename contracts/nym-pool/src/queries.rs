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
        .maybe_grantee_locked(deps.storage, &grantee)?;

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
    use crate::contract::instantiate;
    use crate::testing::{TestSetup, TEST_DENOM};
    use cosmwasm_std::testing::{message_info, mock_dependencies_with_balance, mock_env};
    use cosmwasm_std::{coin, Uint128};
    use nym_pool_contract_common::{Allowance, BasicAllowance, GranterInformation, InstantiateMsg};

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
                    update_granter_set: None,
                },
            )?;

            let updated_admin = query_admin(test.deps())?;
            assert_eq!(updated_admin.admin, Some(new_admin.to_string()));

            Ok(())
        }
    }

    #[test]
    fn available_tokens_query() {
        // no need to test the inner functionalities as this is dealt with in the storage tests
        // (i.e. logic to do with calculating diff against locked tokens, etc.)
        let env = mock_env();
        let mut deps = mock_dependencies_with_balance(&[coin(100, TEST_DENOM)]);

        let init_msg = InstantiateMsg {
            pool_denomination: TEST_DENOM.to_string(),
            grants: Default::default(),
        };

        let some_sender = deps.api.addr_make("some_sender");
        instantiate(
            deps.as_mut(),
            env.clone(),
            message_info(&some_sender, &[]),
            init_msg,
        )
        .unwrap();

        assert_eq!(
            query_available_tokens(deps.as_ref(), env)
                .unwrap()
                .available,
            coin(100, TEST_DENOM)
        );
    }

    #[test]
    fn total_locked_tokens_query() {
        let mut test = TestSetup::init();

        let locked = query_total_locked_tokens(test.deps()).unwrap().locked;
        assert!(locked.amount.is_zero());
        assert_eq!(locked.denom, test.denom());

        let grantee = test.add_dummy_grant().grantee;
        test.lock_allowance(grantee, Uint128::new(1234));

        let locked = query_total_locked_tokens(test.deps()).unwrap().locked;
        assert_eq!(locked.amount, Uint128::new(1234));
        assert_eq!(locked.denom, test.denom());
    }

    #[test]
    fn locked_tokens_query() {
        let mut test = TestSetup::init();

        let grantee1 = test.add_dummy_grant().grantee;
        test.lock_allowance(grantee1.as_str(), Uint128::new(1234));

        let grantee2 = test.add_dummy_grant().grantee;
        let not_grantee = test.generate_account();

        let res = query_locked_tokens(test.deps(), grantee1.to_string()).unwrap();
        assert_eq!(res.grantee, grantee1);
        assert_eq!(res.locked, Some(coin(1234, TEST_DENOM)));

        let res = query_locked_tokens(test.deps(), grantee2.to_string()).unwrap();
        assert_eq!(res.grantee, grantee2);
        assert!(res.locked.is_none());

        let res = query_locked_tokens(test.deps(), not_grantee.to_string()).unwrap();
        assert_eq!(res.grantee, not_grantee);
        assert!(res.locked.is_none());
    }

    #[cfg(test)]
    mod locked_tokens_paged_query {
        use super::*;

        fn lock_sorted(test: &mut TestSetup, count: usize) -> Vec<LockedTokens> {
            let mut grantees = Vec::new();

            for _ in 0..count {
                let grantee = test.add_dummy_grant().grantee;
                test.lock_allowance(grantee.as_str(), Uint128::new(100));
                grantees.push(LockedTokens {
                    grantee,
                    locked: coin(100, test.denom()),
                });
            }

            grantees.sort_by_key(|g| g.grantee.clone());
            grantees
        }

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::init();
            let _locked = lock_sorted(&mut test, 1000);

            let limit = 42;
            let page1 = query_locked_tokens_paged(test.deps(), Some(limit), None).unwrap();
            assert_eq!(page1.locked.len(), limit as usize);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::init();
            let _locked = lock_sorted(&mut test, 1000);

            // query without explicitly setting a limit
            let page1 = query_locked_tokens_paged(test.deps(), None, None).unwrap();
            assert_eq!(
                page1.locked.len() as u32,
                retrieval_limits::LOCKED_TOKENS_DEFAULT_LIMIT
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::init();
            let _locked = lock_sorted(&mut test, 1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 = query_locked_tokens_paged(test.deps(), Some(crazy_limit), None).unwrap();

            assert_eq!(
                page1.locked.len() as u32,
                retrieval_limits::LOCKED_TOKENS_MAX_LIMIT
            );
        }

        #[test]
        fn pagination_works() {
            let mut test = TestSetup::init();
            let locked = lock_sorted(&mut test, 1000);

            // first page should return 2 results...
            let page1 = query_locked_tokens_paged(test.deps(), Some(2), None).unwrap();
            assert_eq!(page1.locked, locked[..2].to_vec());

            // if we start after 5th entry, the returned following page should have 6th and onwards
            let second = locked[1].clone();

            let page2 =
                query_locked_tokens_paged(test.deps(), Some(3), Some(second.grantee.to_string()))
                    .unwrap();
            assert_eq!(page2.locked, locked[2..5].to_vec());
        }
    }

    #[test]
    fn grant_query() {
        let mut test = TestSetup::init();
        let env = test.env();

        // bad address
        let bad_address = "not-valid-bech32";
        assert!(query_grant(test.deps(), env.clone(), bad_address.to_string()).is_err());

        // exists
        let grant = test.add_dummy_grant();
        let grantee = grant.grantee.clone();

        assert_eq!(
            query_grant(test.deps(), env.clone(), grantee.to_string()).unwrap(),
            GrantResponse {
                grantee,
                grant: Some(GrantInformation {
                    grant,
                    expired: false,
                }),
            }
        );

        // exists expired
        let grantee = test.generate_account();
        let exp = env.block.time.seconds() + 1;
        let allowance = Allowance::Basic(BasicAllowance {
            spend_limit: None,
            expiration_unix_timestamp: Some(exp),
        });
        let admin = test.admin_unchecked();
        NYM_POOL_STORAGE
            .insert_new_grant(test.deps_mut(), &env, &admin, &grantee, allowance)
            .unwrap();
        let grant = NYM_POOL_STORAGE.load_grant(test.deps(), &grantee).unwrap();

        test.next_block();
        let env = test.env();

        assert_eq!(
            query_grant(test.deps(), env.clone(), grantee.to_string()).unwrap(),
            GrantResponse {
                grantee,
                grant: Some(GrantInformation {
                    grant,
                    expired: true,
                }),
            }
        );

        // doesn't exist
        let doesnt_exist = test.generate_account();
        assert_eq!(
            query_grant(test.deps(), env.clone(), doesnt_exist.to_string()).unwrap(),
            GrantResponse {
                grantee: doesnt_exist,
                grant: None,
            }
        )
    }

    #[test]
    fn granter_query() {
        let mut test = TestSetup::init();
        let admin = test.admin_unchecked();
        let env = test.env();

        // bad address
        let bad_address = "not-valid-bech32";
        assert!(query_granter(test.deps(), bad_address.to_string()).is_err());

        // exists
        let granter = test.generate_account();
        test.add_granter(&granter);

        assert_eq!(
            query_granter(test.deps(), granter.to_string()).unwrap(),
            GranterResponse {
                granter,
                information: Some(GranterInformation {
                    created_by: admin.clone(),
                    created_at_height: env.block.height,
                }),
            }
        );

        // (admin is also a granter)
        assert_eq!(
            query_granter(test.deps(), admin.to_string()).unwrap(),
            GranterResponse {
                information: Some(GranterInformation {
                    created_by: admin.clone(),
                    created_at_height: env.block.height,
                }),
                granter: admin,
            }
        );

        // doesn't exist
        let not_granter = test.generate_account();
        assert_eq!(
            query_granter(test.deps(), not_granter.to_string()).unwrap(),
            GranterResponse {
                granter: not_granter,
                information: None,
            }
        );
    }

    #[cfg(test)]
    mod granters_paged_query {
        use super::*;

        fn granters_sorted(test: &mut TestSetup, count: usize) -> Vec<GranterDetails> {
            let mut granters = Vec::new();

            for _ in 0..count {
                let granter = test.add_dummy_grant().grantee;
                test.add_granter(&granter);
                granters.push(GranterDetails {
                    granter,
                    information: GranterInformation {
                        created_by: test.admin_unchecked(),
                        created_at_height: test.env().block.height,
                    },
                });
            }

            granters.sort_by_key(|g| g.granter.clone());
            granters
        }

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::init();
            let _granters = granters_sorted(&mut test, 1000);

            let limit = 42;
            let page1 = query_granters_paged(test.deps(), Some(limit), None).unwrap();
            assert_eq!(page1.granters.len(), limit as usize);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::init();
            let _granters = granters_sorted(&mut test, 1000);

            // query without explicitly setting a limit
            let page1 = query_granters_paged(test.deps(), None, None).unwrap();
            assert_eq!(
                page1.granters.len() as u32,
                retrieval_limits::GRANTERS_DEFAULT_LIMIT
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::init();
            let _granters = granters_sorted(&mut test, 1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 = query_granters_paged(test.deps(), Some(crazy_limit), None).unwrap();

            assert_eq!(
                page1.granters.len() as u32,
                retrieval_limits::GRANTERS_MAX_LIMIT
            );
        }

        #[test]
        fn pagination_works() {
            let mut test = TestSetup::init();
            let locked = granters_sorted(&mut test, 1000);

            // first page should return 2 results...
            let page1 = query_granters_paged(test.deps(), Some(2), None).unwrap();
            assert_eq!(page1.granters, locked[..2].to_vec());

            // if we start after 5th entry, the returned following page should have 6th and onwards
            let second = locked[1].clone();

            let page2 =
                query_granters_paged(test.deps(), Some(3), Some(second.granter.to_string()))
                    .unwrap();
            assert_eq!(page2.granters, locked[2..5].to_vec());
        }
    }

    #[cfg(test)]
    mod grants_paged_query {
        use super::*;

        fn grants_sorted(test: &mut TestSetup, count: usize) -> Vec<GrantInformation> {
            let mut grantees = Vec::new();

            for _ in 0..count {
                let grant = test.add_dummy_grant();
                grantees.push(GrantInformation {
                    grant,
                    expired: false,
                });
            }

            grantees.sort_by_key(|g| g.grant.grantee.clone());
            grantees
        }

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::init();
            let _grantees = grants_sorted(&mut test, 1000);

            let limit = 42;
            let page1 = query_grants_paged(test.deps(), test.env(), Some(limit), None).unwrap();
            assert_eq!(page1.grants.len(), limit as usize);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::init();
            let _grantees = grants_sorted(&mut test, 1000);

            // query without explicitly setting a limit
            let page1 = query_grants_paged(test.deps(), test.env(), None, None).unwrap();
            assert_eq!(
                page1.grants.len() as u32,
                retrieval_limits::GRANTS_DEFAULT_LIMIT
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::init();
            let _grantees = grants_sorted(&mut test, 1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 =
                query_grants_paged(test.deps(), test.env(), Some(crazy_limit), None).unwrap();

            assert_eq!(
                page1.grants.len() as u32,
                retrieval_limits::GRANTS_MAX_LIMIT
            );
        }

        #[test]
        fn pagination_works() {
            let mut test = TestSetup::init();
            let grants = grants_sorted(&mut test, 1000);

            // first page should return 2 results...
            let page1 = query_grants_paged(test.deps(), test.env(), Some(2), None).unwrap();
            assert_eq!(page1.grants, grants[..2].to_vec());

            // if we start after 5th entry, the returned following page should have 6th and onwards
            let second = grants[1].clone();

            let page2 = query_grants_paged(
                test.deps(),
                test.env(),
                Some(3),
                Some(second.grant.grantee.to_string()),
            )
            .unwrap();
            assert_eq!(page2.grants, grants[2..5].to_vec());
        }
    }
}
