// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::constants::{
    DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT, DELEGATION_PAGE_MAX_RETRIEVAL_LIMIT,
};
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::Deps;
use cosmwasm_std::Order;
use cosmwasm_std::StdResult;
use cw_storage_plus::Bound;
use mixnet_contract_common::delegation::{MixNodeDelegationResponse, OwnerProxySubKey};
use mixnet_contract_common::{
    delegation, Delegation, MixId, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse,
    PagedMixNodeDelegationsResponse,
};

pub(crate) fn query_mixnode_delegations_paged(
    deps: Deps<'_>,
    mix_id: MixId,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedMixNodeDelegationsResponse> {
    let limit = limit
        .unwrap_or(DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT)
        .min(DELEGATION_PAGE_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(|subkey| {
        Bound::exclusive(Delegation::generate_storage_key_with_subkey(mix_id, subkey))
    });

    let delegations = storage::delegations()
        .idx
        .mixnode
        .prefix(mix_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| record.map(|r| r.1))
        .collect::<StdResult<Vec<Delegation>>>()?;

    let start_next_after = delegations.last().map(|del| del.proxy_storage_key());

    Ok(PagedMixNodeDelegationsResponse::new(
        delegations,
        start_next_after,
    ))
}

pub(crate) fn query_delegator_delegations_paged(
    deps: Deps<'_>,
    delegation_owner: String,
    start_after: Option<(MixId, OwnerProxySubKey)>,
    limit: Option<u32>,
) -> StdResult<PagedDelegatorDelegationsResponse> {
    let validated_owner = deps.api.addr_validate(&delegation_owner)?;

    let limit = limit
        .unwrap_or(DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT)
        .min(DELEGATION_PAGE_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(|(mix_id, subkey)| {
        Bound::exclusive(Delegation::generate_storage_key_with_subkey(mix_id, subkey))
    });

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
        .map(|del| (del.mix_id, del.proxy_storage_key()));

    Ok(PagedDelegatorDelegationsResponse::new(
        delegations,
        start_next_after,
    ))
}

// queries for delegation value of given address for particular node
pub(crate) fn query_mixnode_delegation(
    deps: Deps<'_>,
    mix_id: MixId,
    delegation_owner: String,
    proxy: Option<String>,
) -> StdResult<MixNodeDelegationResponse> {
    let validated_owner = deps.api.addr_validate(&delegation_owner)?;
    let validated_proxy = proxy
        .map(|proxy| deps.api.addr_validate(&proxy))
        .transpose()?;
    let storage_key =
        Delegation::generate_storage_key(mix_id, &validated_owner, validated_proxy.as_ref());

    let delegation = storage::delegations().may_load(deps.storage, storage_key)?;

    let mixnode_still_bonded = mixnodes_storage::mixnode_bonds()
        .may_load(deps.storage, mix_id)?
        .map(|bond| !bond.is_unbonding)
        .unwrap_or_default();

    Ok(MixNodeDelegationResponse::new(
        delegation,
        mixnode_still_bonded,
    ))
}

pub(crate) fn query_all_delegations_paged(
    deps: Deps<'_>,
    start_after: Option<delegation::StorageKey>,
    limit: Option<u32>,
) -> StdResult<PagedAllDelegationsResponse> {
    let limit = limit
        .unwrap_or(DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT)
        .min(DELEGATION_PAGE_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let delegations = storage::delegations()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = delegations.last().map(|del| del.storage_key());

    Ok(PagedAllDelegationsResponse::new(
        delegations,
        start_next_after,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers::TestSetup;

    fn add_dummy_mixes_with_delegations(test: &mut TestSetup, delegators: usize, mixes: usize) {
        for i in 0..mixes {
            let mix_id = test.add_dummy_mixnode(&format!("mix-owner{}", i), None);
            for delegator in 0..delegators {
                let name = &format!("delegator{}", delegator);
                test.add_immediate_delegation(name, 100_000_000u32, mix_id)
            }
        }
    }

    #[cfg(test)]
    mod mixnode_delegations {
        use super::*;
        use crate::support::tests::test_helpers;

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let env = test.env();
            test_helpers::add_dummy_delegations(test.deps_mut(), env, mix_id, 200);

            let limit = 2;

            let page1 =
                query_mixnode_delegations_paged(test.deps(), mix_id, None, Some(limit)).unwrap();
            assert_eq!(limit, page1.delegations.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let env = test.env();
            test_helpers::add_dummy_delegations(test.deps_mut(), env, mix_id, 500);

            // query without explicitly setting a limit
            let page1 = query_mixnode_delegations_paged(test.deps(), mix_id, None, None).unwrap();

            assert_eq!(
                DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let env = test.env();
            test_helpers::add_dummy_delegations(test.deps_mut(), env, mix_id, 5000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 10000;
            let page1 =
                query_mixnode_delegations_paged(test.deps(), mix_id, None, Some(crazy_limit))
                    .unwrap();

            assert_eq!(
                DELEGATION_PAGE_MAX_RETRIEVAL_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            let mut test = TestSetup::new();

            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            test.add_immediate_delegation("addr1", 1000u32, mix_id);

            let per_page = 2;
            let page1 =
                query_mixnode_delegations_paged(test.deps(), mix_id, None, Some(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.delegations.len());

            // save another
            test.add_immediate_delegation("addr2", 1000u32, mix_id);

            // page1 should have 2 results on it
            let page1 =
                query_mixnode_delegations_paged(test.deps(), mix_id, None, Some(per_page)).unwrap();
            assert_eq!(2, page1.delegations.len());

            test.add_immediate_delegation("addr3", 1000u32, mix_id);

            // page1 still has the same 2 results
            let another_page1 =
                query_mixnode_delegations_paged(test.deps(), mix_id, None, Some(per_page)).unwrap();
            assert_eq!(2, another_page1.delegations.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_mixnode_delegations_paged(
                test.deps(),
                mix_id,
                Some(start_after.clone()),
                Some(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.delegations.len());

            // save another one
            test.add_immediate_delegation("addr4", 1000u32, mix_id);

            let page2 = query_mixnode_delegations_paged(
                test.deps(),
                mix_id,
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.delegations.len());
        }

        #[test]
        fn all_retrieved_delegations_are_towards_specified_mixnode() {
            let mut test = TestSetup::new();
            let mix_id1 = test.add_dummy_mixnode("mix-owner1", None);
            let mix_id2 = test.add_dummy_mixnode("mix-owner2", None);
            let mix_id3 = test.add_dummy_mixnode("mix-owner3", None);
            let mix_id4 = test.add_dummy_mixnode("mix-owner4", None);

            let env = test.env();
            // add other "out of order" delegations manually
            test.add_immediate_delegation("random-delegator1", 1000u32, mix_id2);
            test.add_immediate_delegation("random-delegator2", 1000u32, mix_id2);
            test_helpers::add_dummy_delegations(test.deps_mut(), env.clone(), mix_id1, 10);
            test_helpers::add_dummy_delegations(test.deps_mut(), env.clone(), mix_id2, 10);
            test_helpers::add_dummy_delegations(test.deps_mut(), env.clone(), mix_id3, 10);
            test.add_immediate_delegation("random-delegator3", 1000u32, mix_id2);
            test_helpers::add_dummy_delegations(test.deps_mut(), env, mix_id4, 10);
            test.add_immediate_delegation("random-delegator4", 1000u32, mix_id2);

            let res1 = query_mixnode_delegations_paged(test.deps(), mix_id1, None, None).unwrap();
            assert_eq!(res1.delegations.len(), 10);
            assert!(res1.delegations.into_iter().all(|d| d.mix_id == mix_id1));

            let res2 = query_mixnode_delegations_paged(test.deps(), mix_id2, None, None).unwrap();
            assert_eq!(res2.delegations.len(), 14);
            assert!(res2.delegations.into_iter().all(|d| d.mix_id == mix_id2));

            let res3 = query_mixnode_delegations_paged(test.deps(), mix_id3, None, None).unwrap();
            assert_eq!(res3.delegations.len(), 10);
            assert!(res3.delegations.into_iter().all(|d| d.mix_id == mix_id3));

            let res4 = query_mixnode_delegations_paged(test.deps(), mix_id4, None, None).unwrap();
            assert_eq!(res4.delegations.len(), 10);
            assert!(res4.delegations.into_iter().all(|d| d.mix_id == mix_id4));
        }
    }

    mod delegator_delegations {
        use super::*;
        use cosmwasm_std::Addr;

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::new();

            // 50 mixnodes with 500 delegations each;
            add_dummy_mixes_with_delegations(&mut test, 500, 50);

            let limit = 2;

            let page1 = query_delegator_delegations_paged(
                test.deps(),
                "delegator1".into(),
                None,
                Some(limit),
            )
            .unwrap();
            assert_eq!(limit, page1.delegations.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::new();
            add_dummy_mixes_with_delegations(&mut test, 10, 500);

            // query without explicitly setting a limit
            let page1 =
                query_delegator_delegations_paged(test.deps(), "delegator1".into(), None, None)
                    .unwrap();

            assert_eq!(
                DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::new();
            add_dummy_mixes_with_delegations(&mut test, 10, 500);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 10000;
            let page1 = query_delegator_delegations_paged(
                test.deps(),
                "delegator1".into(),
                None,
                Some(crazy_limit),
            )
            .unwrap();

            assert_eq!(
                DELEGATION_PAGE_MAX_RETRIEVAL_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            let mut test = TestSetup::new();

            // note that mix_ids are monotonically increasing
            let mix_id1 = test.add_dummy_mixnode("mix-owner1", None);
            let mix_id2 = test.add_dummy_mixnode("mix-owner2", None);
            let mix_id3 = test.add_dummy_mixnode("mix-owner3", None);
            let mix_id4 = test.add_dummy_mixnode("mix-owner4", None);
            let mix_id5 = test.add_dummy_mixnode("mix-owner5", None);

            // add few delegations from unrelated delegators
            for mix_id in [mix_id1, mix_id2, mix_id3, mix_id4, mix_id5] {
                test.add_immediate_delegation("random1", 1000u32, mix_id);
                test.add_immediate_delegation("random2", 1000u32, mix_id);
                test.add_immediate_delegation("random1", 1000u32, mix_id);
            }

            let delegator = "delegator";

            test.add_immediate_delegation(delegator, 1000u32, mix_id1);

            let per_page = 2;
            let page1 = query_delegator_delegations_paged(
                test.deps(),
                delegator.into(),
                None,
                Some(per_page),
            )
            .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.delegations.len());

            // save another
            test.add_immediate_delegation(delegator, 1000u32, mix_id2);

            // page1 should have 2 results on it
            let page1 = query_delegator_delegations_paged(
                test.deps(),
                delegator.into(),
                None,
                Some(per_page),
            )
            .unwrap();
            assert_eq!(2, page1.delegations.len());

            test.add_immediate_delegation(delegator, 1000u32, mix_id3);

            // page1 still has the same 2 results
            let another_page1 = query_delegator_delegations_paged(
                test.deps(),
                delegator.into(),
                None,
                Some(per_page),
            )
            .unwrap();
            assert_eq!(2, another_page1.delegations.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_delegator_delegations_paged(
                test.deps(),
                delegator.into(),
                Some(start_after.clone()),
                Some(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.delegations.len());

            // save another one
            test.add_immediate_delegation(delegator, 1000u32, mix_id4);

            let page2 = query_delegator_delegations_paged(
                test.deps(),
                delegator.into(),
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.delegations.len());
        }

        #[test]
        fn all_retrieved_delegations_are_from_the_specified_delegator() {
            let mut test = TestSetup::new();

            // it means we have, for example, delegation from "delegator1" towards mix1, mix2, ...., from "delegator2" towards mix1, mix2, ...., etc
            add_dummy_mixes_with_delegations(&mut test, 50, 100);

            // make few queries
            let res1 =
                query_delegator_delegations_paged(test.deps(), "delegator2".into(), None, None)
                    .unwrap();
            assert_eq!(res1.delegations.len(), 100);
            assert!(res1
                .delegations
                .into_iter()
                .all(|d| d.owner == Addr::unchecked("delegator2")));

            let res2 =
                query_delegator_delegations_paged(test.deps(), "delegator35".into(), None, None)
                    .unwrap();
            assert_eq!(res2.delegations.len(), 100);
            assert!(res2
                .delegations
                .into_iter()
                .all(|d| d.owner == Addr::unchecked("delegator35")));
        }
    }

    mod all_delegations {
        use super::*;

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::new();

            // 50 mixnodes with 500 delegations each;
            add_dummy_mixes_with_delegations(&mut test, 500, 50);

            let limit = 2;

            let page1 = query_all_delegations_paged(test.deps(), None, Some(limit)).unwrap();
            assert_eq!(limit, page1.delegations.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::new();
            add_dummy_mixes_with_delegations(&mut test, 10, 500);

            // query without explicitly setting a limit
            let page1 = query_all_delegations_paged(test.deps(), None, None).unwrap();

            assert_eq!(
                DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::new();
            add_dummy_mixes_with_delegations(&mut test, 10, 500);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 10000;
            let page1 = query_all_delegations_paged(test.deps(), None, Some(crazy_limit)).unwrap();

            assert_eq!(
                DELEGATION_PAGE_MAX_RETRIEVAL_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            let mut test = TestSetup::new();

            // note that mix_ids are monotonically increasing and are the first chunk of all
            // delegation storage keys,
            let mix_id1 = test.add_dummy_mixnode("mix-owner1", None);
            let mix_id2 = test.add_dummy_mixnode("mix-owner2", None);

            let delegator1 = "delegator1";
            let delegator2 = "delegator2";

            test.add_immediate_delegation(delegator1, 1000u32, mix_id1);

            let per_page = 2;
            let page1 = query_all_delegations_paged(test.deps(), None, Some(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.delegations.len());
            assert!(
                page1.delegations[0].owner.as_str() == delegator1
                    && page1.delegations[0].mix_id == mix_id1
            );

            test.add_immediate_delegation(delegator1, 1000u32, mix_id2);

            let page1 = query_all_delegations_paged(test.deps(), None, Some(per_page)).unwrap();

            // page1 should have 2 results on it
            assert_eq!(2, page1.delegations.len());
            assert!(
                page1.delegations[0].owner.as_str() == delegator1
                    && page1.delegations[0].mix_id == mix_id1
            );
            assert!(
                page1.delegations[1].owner.as_str() == delegator1
                    && page1.delegations[1].mix_id == mix_id2
            );

            test.add_immediate_delegation(delegator2, 1000u32, mix_id1);

            // note that the order of results changed on page1
            let another_page1 =
                query_all_delegations_paged(test.deps(), None, Some(per_page)).unwrap();
            assert_eq!(2, another_page1.delegations.len());
            assert!(
                another_page1.delegations[0].owner.as_str() == delegator1
                    && another_page1.delegations[0].mix_id == mix_id1
            );
            assert!(
                another_page1.delegations[1].owner.as_str() == delegator2
                    && another_page1.delegations[1].mix_id == mix_id1
            );

            // retrieving the next page should start after the last key on this page
            let start_after = another_page1.start_next_after.unwrap();
            let page2 =
                query_all_delegations_paged(test.deps(), Some(start_after.clone()), Some(per_page))
                    .unwrap();

            assert_eq!(1, page2.delegations.len());
            assert!(
                page2.delegations[0].owner.as_str() == delegator1
                    && page2.delegations[0].mix_id == mix_id2
            );

            // save another one
            test.add_immediate_delegation(delegator2, 1000u32, mix_id2);

            let page2 = query_all_delegations_paged(test.deps(), Some(start_after), Some(per_page))
                .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.delegations.len());
            assert!(
                page2.delegations[0].owner.as_str() == delegator1
                    && page2.delegations[0].mix_id == mix_id2
            );
            assert!(
                page2.delegations[1].owner.as_str() == delegator2
                    && page2.delegations[1].mix_id == mix_id2
            );
        }
    }

    #[cfg(test)]
    mod querying_for_specific_mixnode_delegation {
        use super::*;

        #[test]
        fn when_delegation_doesnt_exist() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let owner = "owner";

            let res = query_mixnode_delegation(test.deps(), mix_id, owner.into(), None).unwrap();
            assert!(res.delegation.is_none());
            assert!(res.mixnode_still_bonded);
        }

        #[test]
        fn when_delegation_exists_but_mixnode_has_unbonded() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let owner = "owner";

            test.add_immediate_delegation(owner, 1000u32, mix_id);
            test.immediately_unbond_mixnode(mix_id);

            let res = query_mixnode_delegation(test.deps(), mix_id, owner.into(), None).unwrap();
            assert_eq!(res.delegation.as_ref().unwrap().owner.as_str(), owner);
            assert_eq!(res.delegation.as_ref().unwrap().amount.amount.u128(), 1000);
            assert!(!res.mixnode_still_bonded);
        }

        #[test]
        fn when_delegation_exists_but_mixnode_is_unbonding() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let owner = "owner";

            test.add_immediate_delegation(owner, 1000u32, mix_id);
            test.start_unbonding_mixnode(mix_id);

            let res = query_mixnode_delegation(test.deps(), mix_id, owner.into(), None).unwrap();
            assert_eq!(res.delegation.as_ref().unwrap().owner.as_str(), owner);
            assert_eq!(res.delegation.as_ref().unwrap().amount.amount.u128(), 1000);
            assert!(!res.mixnode_still_bonded);
        }

        #[test]
        fn when_delegation_exists_with_fully_bonded_node() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let owner = "owner";

            test.add_immediate_delegation(owner, 1000u32, mix_id);

            let res = query_mixnode_delegation(test.deps(), mix_id, owner.into(), None).unwrap();
            assert_eq!(res.delegation.as_ref().unwrap().owner.as_str(), owner);
            assert_eq!(res.delegation.as_ref().unwrap().amount.amount.u128(), 1000);
            assert!(res.mixnode_still_bonded);
        }
    }
}
