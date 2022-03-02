// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use cosmwasm_std::Deps;
use cosmwasm_std::Order;
use cosmwasm_std::StdResult;
use cw_storage_plus::{Bound, PrimaryKey};
use mixnet_contract_common::{
    Delegation, IdentityKey, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse,
    PagedMixDelegationsResponse,
};

pub(crate) fn query_all_network_delegations_paged(
    deps: Deps<'_>,
    start_after: Option<(IdentityKey, String)>,
    limit: Option<u32>,
) -> StdResult<PagedAllDelegationsResponse> {
    let limit = limit
        .unwrap_or(storage::DELEGATION_PAGE_DEFAULT_LIMIT)
        .min(storage::DELEGATION_PAGE_MAX_LIMIT) as usize;

    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::exclusive);

    let delegations = storage::delegations()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| record.map(|r| r.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = delegations
        .last()
        .map(|delegation| delegation.storage_key());

    Ok(PagedAllDelegationsResponse::new(
        delegations,
        start_next_after,
    ))
}

pub(crate) fn query_delegator_delegations_paged(
    deps: Deps<'_>,
    delegation_owner: String,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> StdResult<PagedDelegatorDelegationsResponse> {
    let validated_owner = deps.api.addr_validate(&delegation_owner)?;

    let limit = limit
        .unwrap_or(storage::DELEGATION_PAGE_DEFAULT_LIMIT)
        .min(storage::DELEGATION_PAGE_MAX_LIMIT) as usize;
    let start = start_after
        .map(|mix_identity| Bound::ExclusiveRaw((mix_identity, validated_owner.clone()).joined_key()));

    let delegations = storage::delegations()
        .idx
        .owner
        .prefix(validated_owner)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| record.map(|r| r.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = delegations
        .last()
        .map(|delegation| delegation.node_identity());

    Ok(PagedDelegatorDelegationsResponse::new(
        delegations,
        start_next_after,
    ))
}

// queries for delegation value of given address for particular node
pub(crate) fn query_mixnode_delegation(
    deps: Deps<'_>,
    mix_identity: IdentityKey,
    delegator: String,
) -> Result<Delegation, ContractError> {
    let validated_delegator = deps.api.addr_validate(&delegator)?;
    let storage_key = (mix_identity.clone(), validated_delegator.clone()).joined_key();

    storage::delegations()
        .may_load(deps.storage, storage_key)?
        .ok_or(ContractError::NoMixnodeDelegationFound {
            identity: mix_identity,
            address: validated_delegator,
        })
}

pub(crate) fn query_mixnode_delegations_paged(
    deps: Deps<'_>,
    mix_identity: IdentityKey,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedMixDelegationsResponse> {
    let limit = limit
        .unwrap_or(storage::DELEGATION_PAGE_DEFAULT_LIMIT)
        .min(storage::DELEGATION_PAGE_MAX_LIMIT) as usize;

    let start = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?
        .map(|addr| Bound::ExclusiveRaw((mix_identity.clone(), addr).joined_key()));

    let delegations = storage::delegations()
        .idx
        .mixnode
        .prefix(mix_identity)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| record.map(|r| r.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = delegations.last().map(|delegation| delegation.owner());

    Ok(PagedMixDelegationsResponse::new(
        delegations,
        start_next_after,
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::{coin, Addr, Storage};

    pub fn store_n_mix_delegations(n: u32, storage: &mut dyn Storage, node_identity: &str) {
        for i in 0..n {
            let address = format!("address{}", i);
            test_helpers::save_dummy_delegation(storage, node_identity, address);
        }
    }

    #[cfg(test)]
    mod querying_for_mixnode_delegations_paged {
        use super::*;
        use mixnet_contract_common::IdentityKey;

        #[test]
        fn retrieval_obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let limit = 2;
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(100, &mut deps.storage, &node_identity);

            let page1 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity,
                None,
                Option::from(limit),
            )
            .unwrap();
            assert_eq!(limit, page1.delegations.len() as u32);
        }

        #[test]
        fn retrieval_has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &node_identity,
            );

            // query without explicitly setting a limit
            let page1 =
                query_mixnode_delegations_paged(deps.as_ref(), node_identity, None, None).unwrap();
            assert_eq!(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn retrieval_has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &node_identity,
            );

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000 * storage::DELEGATION_PAGE_DEFAULT_LIMIT;
            let page1 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity,
                None,
                Option::from(crazy_limit),
            )
            .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = storage::DELEGATION_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.delegations.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();

            test_helpers::save_dummy_delegation(&mut deps.storage, &node_identity, "100");

            let per_page = 2;
            let page1 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.delegations.len());

            // save another
            test_helpers::save_dummy_delegation(&mut deps.storage, &node_identity, "200");

            // page1 should have 2 results on it
            let page1 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();
            assert_eq!(2, page1.delegations.len());

            test_helpers::save_dummy_delegation(&mut deps.storage, &node_identity, "300");

            // page1 still has 2 results
            let page1 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();
            assert_eq!(2, page1.delegations.len());
            assert_eq!("200".to_string(), page1.start_next_after.unwrap());

            // retrieving the next page should start after the last key on this page
            let start_after = "200".to_string();
            let page2 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity.clone(),
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.delegations.len());

            // save another one
            test_helpers::save_dummy_delegation(&mut deps.storage, &node_identity, "400");

            let start_after = "200".to_string();
            let page2 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity,
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.delegations.len());
        }
    }

    #[cfg(test)]
    mod querying_for_all_mixnode_delegations_paged {
        use super::*;
        use crate::support::tests::test_helpers;
        use mixnet_contract_common::IdentityKey;

        #[test]
        fn retrieval_obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let limit = 2;
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(100, &mut deps.storage, &node_identity);

            let page1 =
                query_all_network_delegations_paged(deps.as_ref(), None, Option::from(limit))
                    .unwrap();
            assert_eq!(limit, page1.delegations.len() as u32);
        }

        #[test]
        fn retrieval_has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &node_identity,
            );

            // query without explicitly setting a limit
            let page1 = query_all_network_delegations_paged(deps.as_ref(), None, None).unwrap();
            assert_eq!(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn retrieval_has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &node_identity,
            );

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000 * storage::DELEGATION_PAGE_DEFAULT_LIMIT;
            let page1 =
                query_all_network_delegations_paged(deps.as_ref(), None, Option::from(crazy_limit))
                    .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = storage::DELEGATION_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.delegations.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();

            test_helpers::save_dummy_delegation(&mut deps.storage, &node_identity, "100");

            let per_page = 2;
            let page1 =
                query_all_network_delegations_paged(deps.as_ref(), None, Option::from(per_page))
                    .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.delegations.len());

            // save another
            test_helpers::save_dummy_delegation(&mut deps.storage, &node_identity, "200");

            // page1 should have 2 results on it
            let page1 =
                query_all_network_delegations_paged(deps.as_ref(), None, Option::from(per_page))
                    .unwrap();
            assert_eq!(2, page1.delegations.len());

            test_helpers::save_dummy_delegation(&mut deps.storage, &node_identity, "300");

            // page1 still has 2 results
            let page1 =
                query_all_network_delegations_paged(deps.as_ref(), None, Option::from(per_page))
                    .unwrap();
            assert_eq!(2, page1.delegations.len());

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_all_network_delegations_paged(
                deps.as_ref(),
                Option::from(start_after.clone()),
                Option::from(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.delegations.len());

            // save another one
            test_helpers::save_dummy_delegation(&mut deps.storage, &node_identity, "400");

            let page2 = query_all_network_delegations_paged(
                deps.as_ref(),
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.delegations.len());
        }
    }

    #[test]
    fn mix_deletion_query_returns_current_delegation_value() {
        let mut deps = test_helpers::init_contract();
        let node_identity: IdentityKey = "foo".into();
        let delegation_owner = Addr::unchecked("bar");

        let delegation = Delegation::new(
            delegation_owner.clone(),
            node_identity.clone(),
            coin(1234, DENOM),
            1234,
            None,
        );

        storage::delegations()
            .save(
                deps.as_mut().storage,
                delegation.storage_key().joined_key(),
                &delegation,
            )
            .unwrap();

        assert_eq!(
            Ok(delegation),
            query_mixnode_delegation(deps.as_ref(), node_identity, delegation_owner.to_string())
        )
    }

    #[test]
    fn mix_deletion_query_returns_error_if_delegation_doesnt_exist() {
        let mut deps = test_helpers::init_contract();

        let node_identity1: IdentityKey = "foo1".into();
        let node_identity2: IdentityKey = "foo2".into();
        let delegation_owner1 = Addr::unchecked("bar");
        let delegation_owner2 = Addr::unchecked("bar2");

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity1.clone(),
                address: delegation_owner1.clone(),
            }),
            query_mixnode_delegation(
                deps.as_ref(),
                node_identity1.clone(),
                delegation_owner1.to_string()
            )
        );

        // add delegation from a different address
        let delegation = Delegation::new(
            delegation_owner2,
            node_identity1.clone(),
            coin(1234, DENOM),
            1234,
            None,
        );

        storage::delegations()
            .save(
                deps.as_mut().storage,
                delegation.storage_key().joined_key(),
                &delegation,
            )
            .unwrap();

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity1.clone(),
                address: delegation_owner1.clone(),
            }),
            query_mixnode_delegation(
                deps.as_ref(),
                node_identity1.clone(),
                delegation_owner1.to_string()
            )
        );

        // add delegation for a different node
        let delegation = Delegation::new(
            delegation_owner1.clone(),
            node_identity2,
            coin(1234, DENOM),
            1234,
            None,
        );

        storage::delegations()
            .save(
                deps.as_mut().storage,
                delegation.storage_key().joined_key(),
                &delegation,
            )
            .unwrap();

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity1.clone(),
                address: Addr::unchecked(delegation_owner1.clone())
            }),
            query_mixnode_delegation(deps.as_ref(), node_identity1, delegation_owner1.to_string())
        )
    }

    #[cfg(test)]
    mod querying_for_reverse_mixnode_delegations_paged {
        use super::*;

        fn store_n_reverse_delegations(n: u32, storage: &mut dyn Storage, delegation_owner: &str) {
            for i in 0..n {
                let node_identity = format!("node{}", i);
                test_helpers::save_dummy_delegation(storage, node_identity, delegation_owner);
            }
        }

        #[test]
        fn retrieval_obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let limit = 2;
            let delegation_owner = "foo".to_string();
            store_n_reverse_delegations(100, &mut deps.storage, &delegation_owner);

            let page1 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner,
                None,
                Option::from(limit),
            )
            .unwrap();
            assert_eq!(limit, page1.delegations.len() as u32);
        }

        #[test]
        fn retrieval_has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let delegation_owner = "foo".to_string();
            store_n_reverse_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &delegation_owner,
            );

            // query without explicitly setting a limit
            let page1 =
                query_delegator_delegations_paged(deps.as_ref(), delegation_owner, None, None)
                    .unwrap();
            assert_eq!(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn retrieval_has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let delegation_owner = "foo".to_string();
            store_n_reverse_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &delegation_owner,
            );

            // query with a crazy high limit in an attempt to use too many resources
            let crazy_limit = 1000 * storage::DELEGATION_PAGE_DEFAULT_LIMIT;
            let page1 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner,
                None,
                Option::from(crazy_limit),
            )
            .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = storage::DELEGATION_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.delegations.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let mut deps = test_helpers::init_contract();
            let delegation_owner = "bar".to_string();

            test_helpers::save_dummy_delegation(&mut deps.storage, "100", &delegation_owner);

            let per_page = 2;
            let page1 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.delegations.len());

            // save another
            test_helpers::save_dummy_delegation(&mut deps.storage, "200", &delegation_owner);

            // page1 should have 2 results on it
            let page1 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();
            assert_eq!(2, page1.delegations.len());

            test_helpers::save_dummy_delegation(&mut deps.storage, "300", &delegation_owner);

            // page1 still has 2 results
            let page1 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();
            assert_eq!(2, page1.delegations.len());

            // retrieving the next page should start after the last key on this page
            let start_after: IdentityKey = page1.start_next_after.unwrap();
            let page2 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.delegations.len());

            // save another one
            test_helpers::save_dummy_delegation(&mut deps.storage, "400", &delegation_owner);

            let start_after = String::from("2");
            let page2 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner,
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.delegations.len());
        }
    }
}
